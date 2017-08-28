extern crate env_logger;
extern crate piston_window;
#[macro_use] extern crate log;
#[macro_use] extern crate clap;

use std::{io, process};
use std::path::{Path, PathBuf};
use clap::Arg;
use piston_window::{
    OpenGL,
    PistonWindow,
    WindowSettings,
    G2dTexture,
    Viewport,
    Glyphs,
    PressEvent,
    Button,
    Key
};

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
    Piston(PistonError),
}

#[derive(Debug)]
enum PistonError {
    BuildWindow(String),
    LoadFont { file: String, error: piston_window::GlyphError, },
}

fn run() -> Result<(), Error> {
    let matches = app_from_crate!()
        .arg(Arg::with_name("map-file")
             .display_order(1)
             .short("f")
             .long("map-file")
             .value_name("FILE")
             .help("Input map file")
             .default_value("../maps/randomMedium.json")
             .takes_value(true))
        .arg(Arg::with_name("assets-dir")
             .display_order(2)
             .short("a")
             .long("assets-dir")
             .value_name("DIR")
             .help("Graphics resources directory")
             .default_value("./assets")
             .takes_value(true))
        .get_matches();

    let map_file = matches.value_of("map-file")
        .ok_or(Error::MissingParameter("map-file"))?;
    let assets_dir = matches.value_of("assets-dir")
        .ok_or(Error::MissingParameter("assets-dir"))?;

    let opengl = OpenGL::V3_2;
    let mut window: PistonWindow = WindowSettings::new("lambda punter", [640, 480])
        .exit_on_esc(true)
        .opengl(opengl)
        .build()
        .map_err(PistonError::BuildWindow)
        .map_err(Error::Piston)?;

    let mut font_path = PathBuf::from(assets_dir);
    font_path.push("FiraSans-Regular.ttf");
    let mut glyphs = Glyphs::new(&font_path, window.factory.clone())
        .map_err(|e| Error::Piston(PistonError::LoadFont {
            file: font_path.to_string_lossy().to_string(),
            error: e,
        }))?;

    let mut gui_state = GuiState;
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
        });

        if let Some(Button::Keyboard(key)) = event.press_args() {
            gui_state = gui_state.process_key(key)?;
        }

        gui_state = gui_state.tick()?;
    }

    Ok(())
}

struct GuiState;

impl GuiState {
    fn console(&self) -> String {
        "hello lambda punter".to_string()
    }

    fn process_key(self, _key: Key) -> Result<GuiState, Error> {
        Ok(self)
    }

    fn tick(self) -> Result<GuiState, Error> {
        Ok(self)
    }
}

// extern crate sokoban;
// extern crate env_logger;
// extern crate piston_window;
// #[macro_use] extern crate log;
// #[macro_use] extern crate clap;

// use std::{io, thread, process};
// use std::sync::mpsc;
// use std::path::{Path, PathBuf};
// use clap::Arg;
// use sokoban::map::{Coords, Tile, Room};
// use sokoban::game::{Move, Game, GameState};
// use sokoban::solver;
// use piston_window::{
//     OpenGL,
//     PistonWindow,
//     WindowSettings,
//     G2dTexture,
//     Viewport,
//     Glyphs,
//     PressEvent,
//     Button,
//     Key};

// const CONSOLE_HEIGHT: f64 = 32.0;

// fn main() {
//     env_logger::init().unwrap();
//     match run() {
//         Ok(()) =>
//             info!("graceful shutdown"),
//         Err(e) => {
//             error!("fatal error: {:?}", e);
//             process::exit(1);
//         },
//     }
// }

// #[derive(Debug)]
// enum Error {
//     MissingParameter(&'static str),
//     Sokoban(sokoban::Error),
//     Piston(PistonError),
//     SolverThreadSpawn(io::Error),
//     SolverThreadJoin(Box<std::any::Any + Send + 'static>),
//     SolverThreadUnexpectedShutdown,
// }

// #[derive(Debug)]
// enum PistonError {
//     BuildWindow(String),
//     LoadTexture { file: String, error: String, },
//     LoadFont { file: String, error: piston_window::GlyphError, },
// }

// fn run() -> Result<(), Error> {
//     let matches = app_from_crate!()
//         .arg(Arg::with_name("room-file")
//              .display_order(1)
//              .short("f")
//              .long("room-file")
//              .value_name("FILE")
//              .help("Input room map file")
//              .default_value("../sokoban/maps/simple.map")
//              .takes_value(true))
//         .arg(Arg::with_name("assets-dir")
//              .display_order(2)
//              .short("a")
//              .long("assets-dir")
//              .value_name("DIR")
//              .help("Directory with game sprites")
//              .default_value("./assets")
//              .takes_value(true))
//         .get_matches();

//     let assets_dir = matches.value_of("assets-dir")
//         .ok_or(Error::MissingParameter("assets-dir"))?;
//     let room_file = matches.value_of("room-file")
//         .ok_or(Error::MissingParameter("room-file"))?;

//     let opengl = OpenGL::V3_2;
//     let mut window: PistonWindow = WindowSettings::new("sokoban", [640, 480])
//         .exit_on_esc(true)
//         .opengl(opengl)
//         .build()
//         .map_err(PistonError::BuildWindow)
//         .map_err(Error::Piston)?;

//     let textures = load_textures(&mut window, assets_dir)
//         .map_err(Error::Piston)?;

//     let mut font_path = PathBuf::from(assets_dir);
//     font_path.push("FiraSans-Regular.ttf");
//     let mut glyphs = Glyphs::new(&font_path, window.factory.clone())
//         .map_err(|e| Error::Piston(PistonError::LoadFont {
//             file: font_path.to_string_lossy().to_string(),
//             error: e,
//         }))?;

//     let (game, init_state) = sokoban::init_room(room_file)
//         .map_err(Error::Sokoban)?;

//     let mut gui_state = GuiState::Init { game: game, state: init_state, };
//     while let Some(event) = window.next() {
//         window.draw_2d(&event, |context, g2d| {
//             use piston_window::{clear, Image, text, Transformed};
//             clear([0.0, 0.0, 0.0, 1.0], g2d);
//             text::Text::new_color([0.0, 1.0, 0.0, 1.0], 16).draw(
//                 &gui_state.console(),
//                 &mut glyphs,
//                 &context.draw_state,
//                 context.transform.trans(5.0, 20.0),
//                 g2d
//             );

//             draw_scene(gui_state.get_state(), |coords, room, sprite| {
//                 Image::new()
//                     .rect(translate_tile_coords(&context.viewport, room, coords))
//                     .draw(match sprite {
//                         Sprite::Tile(&Tile::Wall) => &textures.wall,
//                         Sprite::Tile(&Tile::Floor) => &textures.floor,
//                         Sprite::Tile(&Tile::CrateDst) => &textures.dst,
//                         Sprite::Player => &textures.player,
//                         Sprite::Crate => &textures.crate_,
//                     }, &Default::default(), context.transform, g2d);
//             });
//         });

//         if let Some(Button::Keyboard(key)) = event.press_args() {
//             gui_state = gui_state.process_key(key)?;
//         }

//         gui_state = gui_state.tick()?;
//     }

//     Ok(())
// }

// struct DebugStep {
//     moves: Vec<Move>,
//     state: GameState,
//     finished: bool,
//     reachable_tiles_count: usize,
//     crates_on_dst_count: usize,
//     nearest_crate_nearest_dst_sq_dist: usize,
//     nearest_crate_sq_dist: usize,
// }

// enum DebugPacket {
//     Step(DebugStep),
//     Done(Game, Option<Vec<(Move, GameState)>>),
// }

// enum DebugStepMode {
//     WantRecv,
//     WantSend,
// }

// enum GuiState {
//     Init {
//         game: Game,
//         state: GameState,
//     },
//     Solving {
//         state: GameState,
//         rx: mpsc::Receiver<(Game, Option<Vec<(Move, GameState)>>)>,
//         handle: thread::JoinHandle<()>,
//     },
//     NoSolution(GameState),
//     Solution {
//         solution: Vec<GameState>,
//         step: usize,
//     },
//     DebugSolve {
//         mode: DebugStepMode,
//         step: DebugStep,
//         tx: mpsc::Sender<()>,
//         rx: mpsc::Receiver<DebugPacket>,
//         handle: thread::JoinHandle<()>,
//     },
// }

// impl GuiState {
//     fn console(&self) -> String {
//         match self {
//             &GuiState::Init { .. } =>
//                 "Map loaded. Press <S> to solve or <D> to solve in debug mode.".to_string(),
//             &GuiState::Solving { .. } =>
//                 "Solving, please wait...".to_string(),
//             &GuiState::NoSolution(..) =>
//                 "No solution found :( Press <ESC> to quit.".to_string(),
//             &GuiState::Solution { ref solution, step, } =>
//                 format!("Solution found: step {} of {}. Press <N> for next step or <P> for previous.",
//                         step + 1, solution.len()),
//             &GuiState::DebugSolve { ref mode, ref step, .. } =>
//                 format!("Inspecting state: finished: {}, rtc/cod/ncnd/nc: {}/{}/{}/{}.{} Path so far: {:?}",
//                         step.finished,
//                         step.reachable_tiles_count,
//                         step.crates_on_dst_count,
//                         step.nearest_crate_nearest_dst_sq_dist,
//                         step.nearest_crate_sq_dist,
//                         match mode {
//                             &DebugStepMode::WantRecv => "",
//                             &DebugStepMode::WantSend => " Press <N> for next step.",
//                         },
//                         step.moves),
//         }
//     }

//     fn get_state(&self) -> &GameState {
//         match self {
//             &GuiState::Init { state: ref s, .. } => s,
//             &GuiState::Solving { state: ref s, .. } => s,
//             &GuiState::NoSolution(ref s) => s,
//             &GuiState::Solution { ref solution, step, } => &solution[step],
//             &GuiState::DebugSolve { ref step, .. } => &step.state,
//         }
//     }

//     fn process_key(self, key: Key) -> Result<GuiState, Error> {
//         Ok(match (self, key) {
//             (GuiState::Init { mut game, state, }, Key::S) => {
//                 let (tx, rx) = mpsc::channel();
//                 let init_state = state.clone();
//                 let handle = thread::Builder::new()
//                     .name("solver background thread".to_string())
//                     .spawn(move || {
//                         let solution = solver::a_star::solve(&mut game, init_state);
//                         tx.send((game, solution)).ok();
//                     })
//                     .map_err(Error::SolverThreadSpawn)?;
//                 GuiState::Solving {
//                     state: state,
//                     rx: rx,
//                     handle: handle,
//                 }
//             },
//             (GuiState::Init { mut game, state, }, Key::D) => {
//                 let (master_tx, slave_rx) = mpsc::channel();
//                 let (slave_tx, master_rx) = mpsc::channel();
//                 let init_state = state.clone();
//                 let handle = thread::Builder::new()
//                     .name("debug solver background thread".to_string())
//                     .spawn(move || {
//                         let solution = solver::a_star::solve_debug(&mut game, init_state, |moves, st, finished, rtc, cod, ncnd, nc| {
//                             slave_tx.send(DebugPacket::Step(DebugStep {
//                                 moves: moves.iter().map(|v| v.0).collect(),
//                                 state: st.clone(),
//                                 finished: finished,
//                                 reachable_tiles_count: rtc,
//                                 crates_on_dst_count: cod,
//                                 nearest_crate_nearest_dst_sq_dist: ncnd,
//                                 nearest_crate_sq_dist: nc,
//                             })).ok();
//                             slave_rx.recv().ok();
//                         });
//                         slave_tx.send(DebugPacket::Done(game, solution)).ok();
//                     })
//                     .map_err(Error::SolverThreadSpawn)?;
//                 GuiState::DebugSolve {
//                     mode: DebugStepMode::WantRecv,
//                     step: DebugStep {
//                         moves: vec![],
//                         state: state,
//                         finished: false,
//                         reachable_tiles_count: 0,
//                         crates_on_dst_count: 0,
//                         nearest_crate_nearest_dst_sq_dist: 0,
//                         nearest_crate_sq_dist: 0,
//                     },
//                     tx: master_tx,
//                     rx: master_rx,
//                     handle: handle,
//                 }
//             },
//             (GuiState::Solution { solution, step, }, Key::P) =>
//                 GuiState::Solution {
//                     step: if step == 0 { 0 } else { step - 1 },
//                     solution: solution,
//                 },
//             (GuiState::Solution { solution, step, }, Key::N) =>
//                 GuiState::Solution {
//                     step: if step + 1 >= solution.len() { step } else { step + 1 },
//                     solution: solution,
//                 },
//             (GuiState::DebugSolve { mode: DebugStepMode::WantSend, step, tx, rx, handle, }, Key::N) => {
//                 tx.send(()).map_err(|_| Error::SolverThreadUnexpectedShutdown)?;
//                 GuiState::DebugSolve {
//                     mode: DebugStepMode::WantRecv,
//                     step: step,
//                     tx: tx,
//                     rx: rx,
//                     handle: handle,
//                 }
//             },
//             (other, _) =>
//                 other,
//         })
//     }

//     fn tick(self) -> Result<GuiState, Error> {
//         match self {
//             GuiState::Solving { state, rx, handle } =>
//                 match rx.try_recv() {
//                     Ok((_, None)) => {
//                         let () = handle.join().map_err(Error::SolverThreadJoin)?;
//                         Ok(GuiState::NoSolution(state))
//                     },
//                     Ok((_game, Some(solution))) => {
//                         let () = handle.join().map_err(Error::SolverThreadJoin)?;
//                         let mut full_solution = Vec::with_capacity(solution.len() + 1);
//                         full_solution.push(state);
//                         full_solution.extend(solution.into_iter().map(|v| v.1));
//                         Ok(GuiState::Solution {
//                             solution: full_solution,
//                             step: 0,
//                         })
//                     },
//                     Err(mpsc::TryRecvError::Empty) =>
//                         Ok(GuiState::Solving {
//                             state: state,
//                             rx: rx,
//                             handle: handle,
//                         }),
//                     Err(mpsc::TryRecvError::Disconnected) =>
//                         Err(Error::SolverThreadUnexpectedShutdown),
//                 },
//             GuiState::DebugSolve { mode: DebugStepMode::WantRecv, step, tx, rx, handle, } =>
//                 match rx.try_recv() {
//                     Ok(DebugPacket::Step(next_step)) =>
//                         Ok(GuiState::DebugSolve {
//                             mode: DebugStepMode::WantSend,
//                             step: next_step,
//                             tx: tx,
//                             rx: rx,
//                             handle: handle,
//                         }),
//                     Ok(DebugPacket::Done(_, None)) => {
//                         let () = handle.join().map_err(Error::SolverThreadJoin)?;
//                         Ok(GuiState::NoSolution(step.state))
//                     },
//                     Ok(DebugPacket::Done(_game, Some(solution))) => {
//                         let () = handle.join().map_err(Error::SolverThreadJoin)?;
//                         Ok(GuiState::Solution {
//                             solution: solution.into_iter().map(|v| v.1).collect(),
//                             step: 0,
//                         })
//                     },
//                     Err(mpsc::TryRecvError::Empty) =>
//                         Ok(GuiState::DebugSolve {
//                             mode: DebugStepMode::WantRecv,
//                             step: step,
//                             tx: tx,
//                             rx: rx,
//                             handle: handle,
//                         }),
//                     Err(mpsc::TryRecvError::Disconnected) =>
//                         Err(Error::SolverThreadUnexpectedShutdown),
//                 },
//             other =>
//                 Ok(other),
//         }
//     }
// }

// enum Sprite<'a> {
//     Tile(&'a Tile),
//     Player,
//     Crate,
// }

// fn draw_scene<DS>(state: &GameState, mut draw_sprite: DS) where DS: for<'a> FnMut(&Coords, &Room, Sprite<'a>) {
//     for (i, tile) in state.room.content.iter().enumerate() {
//         let row = i / state.room.width;
//         let col = i % state.room.width;
//         draw_sprite(&(row as isize, col as isize), &state.room, Sprite::Tile(tile));
//     }
//     draw_sprite(&state.placement.player, &state.room, Sprite::Player);
//     for crate_coord in state.placement.crates.iter() {
//         draw_sprite(crate_coord, &state.room, Sprite::Crate);
//     }
// }

// fn translate_tile_coords(viewport: &Option<Viewport>, room: &Room, &(row, cell): &Coords) -> [f64; 4] {
//     let (w, h) = viewport
//         .map(|v| (v.draw_size[0], v.draw_size[1]))
//         .unwrap_or((640, 480));
//     let tile_width = w as f64 / room.width as f64;
//     let tile_height = (h as f64 - CONSOLE_HEIGHT) / room.height as f64;
//     let tile_side = if tile_width < tile_height {
//         tile_width
//     } else {
//         tile_height
//     };
//     [cell as f64 * tile_side, row as f64 * tile_side + CONSOLE_HEIGHT, tile_side, tile_side]
// }

// struct SokobanTextures {
//     crate_: G2dTexture,
//     dst: G2dTexture,
//     floor: G2dTexture,
//     player: G2dTexture,
//     wall: G2dTexture,
// }

// fn load_textures<P>(window: &mut PistonWindow, assets_dir: P) -> Result<SokobanTextures, PistonError> where P: AsRef<Path> {
//     let mut path = PathBuf::from(assets_dir.as_ref());
//     let mut load_texture = |filename| {
//         use piston_window::{Texture, TextureSettings, Flip};
//         path.push(filename);
//         let t = Texture::from_path(&mut window.factory, &path, Flip::None, &TextureSettings::new())
//             .map_err(|e| PistonError::LoadTexture {
//             file: path.to_string_lossy().to_string(),
//             error: e,
//             });
//         path.pop();
//         t
//     };

//     Ok(SokobanTextures {
//         crate_: load_texture("crate.png")?,
//         dst: load_texture("dst.png")?,
//         floor: load_texture("floor.png")?,
//         player: load_texture("player.png")?,
//         wall: load_texture("wall.png")?,
//     })
// }
