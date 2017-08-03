extern crate sokoban;
extern crate env_logger;
extern crate piston_window;
#[macro_use] extern crate log;
#[macro_use] extern crate clap;

use std::process;
use std::path::{Path, PathBuf};
use clap::Arg;
use piston_window::{OpenGL, PistonWindow, WindowSettings, G2dTexture};
//use sokoban::{game, solver};

fn main() {
    env_logger::init().unwrap();
    match run() {
        Ok(()) =>
            info!("graceful shutdown"),
        Err(e) => {
            error!("fatal error: {:?}", e);
            process::exit(1);
        },
    }
}

#[derive(Debug)]
enum Error {
    MissingParameter(&'static str),
    Sokoban(sokoban::Error),
    Piston(PistonError),
}

#[derive(Debug)]
enum PistonError {
    BuildWindow(String),
    LoadTexture { file: String, error: String, },
}

fn run() -> Result<(), Error> {
    let matches = app_from_crate!()
        .arg(Arg::with_name("room-file")
             .display_order(1)
             .short("f")
             .long("room-file")
             .value_name("FILE")
             .help("Input room map file")
             .default_value("../sokoban/maps/simple.map")
             .takes_value(true))
        .arg(Arg::with_name("assets-dir")
             .display_order(2)
             .short("a")
             .long("assets-dir")
             .value_name("DIR")
             .help("Directory with game sprites")
             .default_value("./assets")
             .takes_value(true))
        .get_matches();

    let assets_dir = matches.value_of("assets-dir")
        .ok_or(Error::MissingParameter("assets-dir"))?;
    let room_file = matches.value_of("room-file")
        .ok_or(Error::MissingParameter("room-file"))?;

    let opengl = OpenGL::V3_2;
    let mut window: PistonWindow = WindowSettings::new("sokoban", [640, 480])
        .exit_on_esc(true)
        .opengl(opengl)
        .build()
        .map_err(PistonError::BuildWindow)
        .map_err(Error::Piston)?;

    let textures = load_textures(&mut window, assets_dir)
        .map_err(Error::Piston)?;

    let (mut game, init_state) = sokoban::init_room(room_file)
        .map_err(Error::Sokoban)?;

    while let Some(event) = window.next() {
        window.draw_2d(&event, |context, g2d| {
            use piston_window::{clear, image, Image};
            clear([1.0; 4], g2d);
            Image::new()
                .rect([0.0, 0.0, 32.0, 32.0])
                .draw(&textures.crate_, &Default::default(), context.transform, g2d);
            Image::new()
                .rect([32.0, 0.0, 32.0, 32.0])
                .draw(&textures.dst, &Default::default(), context.transform, g2d);
            Image::new()
                .rect([64.0, 0.0, 32.0, 32.0])
                .draw(&textures.floor, &Default::default(), context.transform, g2d);
            Image::new()
                .rect([0.0, 32.0, 32.0, 32.0])
                .draw(&textures.player, &Default::default(), context.transform, g2d);
            Image::new()
                .rect([32.0, 32.0, 32.0, 32.0])
                .draw(&textures.wall, &Default::default(), context.transform, g2d);

        });
    }

    Ok(())
}

struct SokobanTextures {
    crate_: G2dTexture,
    dst: G2dTexture,
    floor: G2dTexture,
    player: G2dTexture,
    wall: G2dTexture,
}

fn load_textures<P>(window: &mut PistonWindow, assets_dir: P) -> Result<SokobanTextures, PistonError> where P: AsRef<Path> {
    let mut path = PathBuf::from(assets_dir.as_ref());
    let mut load_texture = |filename| {
        use piston_window::{Texture, TextureSettings, Flip};
        path.push(filename);
        let t = Texture::from_path(&mut window.factory, &path, Flip::None, &TextureSettings::new())
            .map_err(|e| PistonError::LoadTexture {
            file: path.to_string_lossy().to_string(),
            error: e,
            });
        path.pop();
        t
    };

    Ok(SokobanTextures {
        crate_: load_texture("crate.png")?,
        dst: load_texture("dst.png")?,
        floor: load_texture("floor.png")?,
        player: load_texture("player.png")?,
        wall: load_texture("wall.png")?,
    })
}
