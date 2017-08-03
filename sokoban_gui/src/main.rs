extern crate sokoban;
extern crate env_logger;
extern crate piston_window;
#[macro_use] extern crate log;
#[macro_use] extern crate clap;

use std::{process};
use std::path::{Path, PathBuf};
use clap::Arg;
use sokoban::map::{Coords, Tile, Room};
use sokoban::game::{Game, GameState};
use piston_window::{
    OpenGL,
    PistonWindow,
    WindowSettings,
    G2dTexture,
    Viewport,
    Glyphs,
    PressEvent,
    Button,
    Key};

const CONSOLE_HEIGHT: f64 = 32.0;

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
    LoadFont { file: String, error: piston_window::GlyphError, },
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

    let mut font_path = PathBuf::from(assets_dir);
    font_path.push("FiraSans-Regular.ttf");
    let mut glyphs = Glyphs::new(&font_path, window.factory.clone())
        .map_err(|e| Error::Piston(PistonError::LoadFont {
            file: font_path.to_string_lossy().to_string(),
            error: e,
        }))?;

    let (mut game, init_state) = sokoban::init_room(room_file)
        .map_err(Error::Sokoban)?;

    let mut gui_state = GuiState::Init(init_state);
    while let Some(event) = window.next() {
        window.draw_2d(&event, |context, g2d| {
            use piston_window::{clear, Image, text, Transformed};
            clear([0.0, 0.0, 0.0, 1.0], g2d);
            text::Text::new_color([0.0, 1.0, 0.0, 1.0], 16).draw(
                &gui_state.console(),
                &mut glyphs,
                &context.draw_state,
                context.transform.trans(5.0, 20.0),
                g2d
            );

            draw_scene(gui_state.get_state(), |coords, room, sprite| {
                Image::new()
                    .rect(translate_tile_coords(&context.viewport, room, coords))
                    .draw(match sprite {
                        Sprite::Tile(&Tile::Wall) => &textures.wall,
                        Sprite::Tile(&Tile::Floor) => &textures.floor,
                        Sprite::Tile(&Tile::CrateDst) => &textures.dst,
                        Sprite::Player => &textures.player,
                        Sprite::Crate => &textures.crate_,
                    }, &Default::default(), context.transform, g2d);
            });
        });

        if let Some(Button::Keyboard(key)) = event.press_args() {
            gui_state = gui_state.process_key(key, &mut game);
        }
    }

    Ok(())
}

enum GuiState {
    Init(GameState),
    Solving(GameState),
}

impl GuiState {
    fn console(&self) -> String {
        match self {
            &GuiState::Init(..) =>
                "Map loaded. Press <s> to solve.".to_string(),
            &GuiState::Solving(..) =>
                "Solving...".to_string(),
        }
    }

    fn get_state(&self) -> &GameState {
        match self {
            &GuiState::Init(ref s) => s,
            &GuiState::Solving(ref s) => s,
        }
    }

    fn process_key(self, key: Key, game: &mut Game) -> GuiState {
        match (self, key) {
            (GuiState::Init(state), Key::S) => {
                GuiState::Solving(state)
            },
            (other, _) =>
                other,
        }
    }
}

enum Sprite<'a> {
    Tile(&'a Tile),
    Player,
    Crate,
}

fn draw_scene<DS>(state: &GameState, mut draw_sprite: DS) where DS: for<'a> FnMut(&Coords, &Room, Sprite<'a>) {
    for (i, tile) in state.room.content.iter().enumerate() {
        let row = i / state.room.width;
        let col = i % state.room.width;
        draw_sprite(&(row as isize, col as isize), &state.room, Sprite::Tile(tile));
    }
    draw_sprite(&state.placement.player, &state.room, Sprite::Player);
    for crate_coord in state.placement.crates.iter() {
        draw_sprite(crate_coord, &state.room, Sprite::Crate);
    }
}

fn translate_tile_coords(viewport: &Option<Viewport>, room: &Room, &(row, cell): &Coords) -> [f64; 4] {
    let (w, h) = viewport
        .map(|v| (v.draw_size[0], v.draw_size[1]))
        .unwrap_or((640, 480));
    let tile_width = w as f64 / room.width as f64;
    let tile_height = (h as f64 - CONSOLE_HEIGHT) / room.height as f64;
    let tile_side = if tile_width < tile_height {
        tile_width
    } else {
        tile_height
    };
    [cell as f64 * tile_side, row as f64 * tile_side + CONSOLE_HEIGHT, tile_side, tile_side]
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