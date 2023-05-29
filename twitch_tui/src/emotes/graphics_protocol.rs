use anyhow::{anyhow, Context};
use base64::{engine::general_purpose::STANDARD, Engine};
use crossterm::{csi, cursor::MoveTo, queue, Command};
use dialoguer::console::{Key, Term};
use image::{
    codecs::{gif::GifDecoder, webp::WebPDecoder},
    io::Reader,
    AnimationDecoder, ImageDecoder, ImageFormat,
};
use std::{
    env, fmt, fs,
    fs::File,
    io::{BufReader, Write},
    mem,
    path::PathBuf,
};

use crate::{
    handlers::data::EmoteData,
    utils::pathing::{
        create_temp_file, pathbuf_try_to_string, remove_temp_file, save_in_temp_file,
    },
};

/// Macro to add the graphics protocol escape sequence around a command.
/// See <https://sw.kovidgoyal.net/kitty/graphics-protocol/> for documentation of the terminal graphics protocol
macro_rules! gp {
    ($c:expr) => {
        concat!("\x1B_G", $c, "\x1b\\")
    };
}

/// The temporary files created for the graphics protocol need to have the `tty-graphics-protocol`
/// string to be deleted by the terminal.
const GP_PREFIX: &str = "twt.tty-graphics-protocol.";

type Result<T = ()> = anyhow::Result<T>;

pub trait Size {
    fn size(&self) -> (u32, u32);
}

pub struct StaticImage {
    id: u32,
    width: u32,
    height: u32,
    path: PathBuf,
}

impl StaticImage {
    pub fn new(id: u32, image: Reader<BufReader<File>>) -> Result<Self> {
        let image = image.decode()?.to_rgba8();
        let (width, height) = image.dimensions();
        let (mut tempfile, pathbuf) = create_temp_file(GP_PREFIX)?;
        if let Err(e) = save_in_temp_file(image.as_raw(), &mut tempfile) {
            remove_temp_file(&pathbuf);
            return Err(e);
        }

        Ok(Self {
            id,
            width,
            height,
            path: pathbuf,
        })
    }
}

impl Command for StaticImage {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        write!(
            f,
            gp!("a=t,t=t,f=32,s={width},v={height},i={id},q=2;{path}"),
            width = self.width,
            height = self.height,
            id = self.id,
            path = STANDARD.encode(pathbuf_try_to_string(&self.path).map_err(|_| fmt::Error)?)
        )
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> std::result::Result<(), std::io::Error> {
        panic!("Windows version not supported.")
    }
}

impl Size for StaticImage {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

pub struct AnimatedImage {
    id: u32,
    width: u32,
    height: u32,
    frames: Vec<(PathBuf, u32)>,
}

impl AnimatedImage {
    pub fn new<'a>(id: u32, decoder: impl ImageDecoder<'a> + AnimationDecoder<'a>) -> Result<Self> {
        let (width, height) = decoder.dimensions();
        let frames = decoder.into_frames().collect_frames()?;
        let iter = frames.iter();

        let (ok, err): (Vec<_>, Vec<_>) = iter
            .map(|f| {
                let (mut tempfile, pathbuf) = create_temp_file(GP_PREFIX)?;
                save_in_temp_file(f.buffer().as_raw(), &mut tempfile)?;
                let delay = f.delay().numer_denom_ms().0;

                Ok((pathbuf, delay))
            })
            .partition(Result::is_ok);

        let frames: Vec<(PathBuf, u32)> = ok.into_iter().filter_map(Result::ok).collect();

        // If we had any error, we need to delete the temp files, as the terminal won't do it for us.
        if !err.is_empty() {
            for (path, _) in &frames {
                mem::drop(fs::remove_file(path));
            }
            return Err(anyhow!("Invalid frame in gif."));
        }

        if frames.is_empty() {
            Err(anyhow!("Image has no frames"))
        } else {
            Ok(Self {
                id,
                width,
                height,
                frames,
            })
        }
    }
}

impl Command for AnimatedImage {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        if self.frames.is_empty() {
            return Err(fmt::Error);
        }

        let mut frames = self.frames.iter();

        // We need to send the data for the first frame as a normal image.
        // We can unwrap here because we checked above if frames was empty.
        let (path, delay) = frames.next().unwrap();
        write!(
            f,
            gp!("a=t,t=t,f=32,s={width},v={height},i={id},q=2;{path}"),
            id = self.id,
            width = self.width,
            height = self.height,
            path = STANDARD.encode(pathbuf_try_to_string(path).map_err(|_| fmt::Error)?)
        )?;
        // r=1: First frame
        write!(
            f,
            gp!("a=a,i={id},r=1,z={delay},q=2;"),
            id = self.id,
            delay = delay,
        )?;

        for (path, delay) in frames {
            write!(
                f,
                gp!("a=f,t=t,f=32,s={width},v={height},i={id},z={delay},q=2;{path}"),
                id = self.id,
                width = self.width,
                height = self.height,
                delay = delay,
                path = STANDARD.encode(pathbuf_try_to_string(path).map_err(|_| fmt::Error)?)
            )?;
        }

        // s=3: Start animation, v=1: Loop infinitely
        write!(f, gp!("a=a,i={id},s=3,v=1,q=2;"), id = self.id)
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> std::result::Result<(), std::io::Error> {
        panic!("Windows version not supported.")
    }
}

impl Size for AnimatedImage {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

pub enum Load {
    Static(StaticImage),
    Animated(AnimatedImage),
}

impl Load {
    pub fn new(id: u32, path: &str) -> Result<Self> {
        let path = std::path::PathBuf::from(path);
        let image = Reader::open(&path)?.with_guessed_format()?;

        match image.format() {
            None => Err(anyhow!("Could not guess image format.")),
            Some(ImageFormat::WebP) => {
                let decoder = WebPDecoder::new(image.into_inner())?;

                if decoder.has_animation() {
                    // Some animated webp images have a default white background color
                    // We replace it by a transparent background
                    // TODO: uncomment this line once PR below is merged.
                    // https://github.com/image-rs/image/pull/1907
                    // decoder.set_background_color(Rgba([0, 0, 0, 0]))?;
                    Ok(Self::Animated(AnimatedImage::new(id, decoder)?))
                } else {
                    let image = Reader::open(&path)?.with_guessed_format()?;

                    Ok(Self::Static(StaticImage::new(id, image)?))
                }
            }
            Some(ImageFormat::Gif) => {
                let decoder = GifDecoder::new(image.into_inner())?;
                Ok(Self::Animated(AnimatedImage::new(id, decoder)?))
            }
            Some(_) => Ok(Self::Static(StaticImage::new(id, image)?)),
        }
    }
}

impl Command for Load {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        match self {
            Self::Static(s) => s.write_ansi(f),
            Self::Animated(a) => a.write_ansi(f),
        }
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> std::result::Result<(), std::io::Error> {
        panic!("Windows version not supported.")
    }
}

impl Size for Load {
    fn size(&self) -> (u32, u32) {
        match self {
            Self::Static(s) => s.size(),
            Self::Animated(a) => a.size(),
        }
    }
}

pub struct Display {
    x: u16,
    y: u16,
    id: u32,
    pid: u32,
    width: u16,
    offset: u16,
    layer: u16,
}

impl Display {
    pub const fn new(
        (x, y): (u16, u16),
        EmoteData { id, pid, layer, .. }: &EmoteData,
        width: u16,
        offset: u16,
    ) -> Self {
        Self {
            x,
            y,
            id: *id,
            pid: *pid,
            width,
            offset,
            layer: *layer,
        }
    }
}

impl Command for Display {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        MoveTo(self.x, self.y).write_ansi(f)?;
        // r=1: Set height to 1 row
        write!(
            f,
            gp!("a=p,i={id},p={pid},r=1,c={width},X={offset},z={z},q=2;"),
            id = self.id,
            pid = self.pid,
            width = self.width,
            offset = self.offset,
            z = self.layer
        )
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> std::result::Result<(), std::io::Error> {
        panic!("Windows version not supported.")
    }
}

#[derive(Eq, PartialEq)]
pub struct Clear(pub u32, pub u32);

impl Command for Clear {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        if *self == Self(0, 0) {
            // Delete and unload all images
            write!(f, gp!("a=d,d=A,q=2;"))
        } else if self.0 == 0 {
            // Delete all images
            write!(f, gp!("a=d,d=a,q=2;"))
        } else {
            write!(
                f,
                gp!("a=d,d=i,i={id},p={pid},q=2;"),
                id = self.0,
                pid = self.1,
            )
        }
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> std::result::Result<(), std::io::Error> {
        panic!("Windows version not supported.")
    }
}

/// Send a csi query to the terminal. The terminal must respond in the format `<ESC>[(a)(r)(c)`,
/// where `(a)` can be any character, `(r)` is the terminal response, and `(c)` is the last character of the query.
/// If the terminal does not respond, or responds in a different format, this function will cause an infinite loop.
/// See [here](https://www.xfree86.org/current/ctlseqs.html) for information about xterm control sequences.
/// This function will strip the `<ESC>[(a)` and the last char `(c)` and return the response `(r)`.
fn query_terminal(command: &[u8]) -> Result<String> {
    let c = *command.last().context("Command is empty")? as char;
    let mut stdout = Term::stdout();
    stdout.write_all(command)?;
    stdout.flush()?;

    // Empty stdout buffer until we find the terminal response.
    loop {
        let c = stdout.read_key()?;
        if let Key::UnknownEscSeq(_) = c {
            break;
        }
    }

    let mut response = String::new();
    loop {
        match stdout.read_key() {
            Ok(Key::Char(chr)) if chr == c => break,
            Ok(Key::Char(chr)) => response.push(chr),
            Err(_) => break,
            _ => (),
        }
    }
    Ok(response)
}

pub fn get_terminal_cell_size() -> Result<(u16, u16)> {
    // Request the terminal size in pixels.
    let res = query_terminal(csi!("14t").as_bytes())?;

    // Response has the format <height>;<width>
    let mut values = res.split(';');
    let height_px = values
        .next()
        .context("Invalid response from terminal")?
        .parse::<u16>()?;
    let width_px = values
        .next()
        .context("Invalid response from terminal")?
        .parse::<u16>()?;

    // Size of terminal: (columns, rows)
    let (ncols, nrows) = crossterm::terminal::size()?;

    Ok((width_px / ncols, height_px / nrows))
}

/// First check if the terminal is `kitty` or `WezTerm`, theses are the only terminals that fully support the graphics protocol as of 2023-04-09.
/// Then check that it supports the graphics protocol using temporary files, by sending a graphics protocol request followed by a request for terminal attributes.
/// If we receive the terminal attributes without receiving the response for the graphics protocol, it does not support it.
pub fn support_graphics_protocol() -> Result<bool> {
    Ok(
        (env::var("TERM")? == "xterm-kitty" || env::var("TERM_PROGRAM")? == "WezTerm")
            && query_terminal(
                format!(
                    concat!(gp!("i=31,s=1,v=1,a=q,t=d,f=24;{}"), csi!("c")),
                    STANDARD.encode("AAAA"),
                )
                .as_bytes(),
            )?
            .contains("OK"),
    )
}

pub fn command(c: impl Command) -> Result {
    Ok(queue!(std::io::stdout(), c)?)
}
