extern crate serde;
extern crate serde_json;
extern crate env_logger;
extern crate piston_window;
extern crate lambda_punter as lp;
#[macro_use] extern crate log;
#[macro_use] extern crate clap;
#[macro_use] extern crate serde_derive;

use std::{io, fs, thread, process};
use std::sync::{mpsc, Arc};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use clap::Arg;
use piston_window::{
    OpenGL,
    PistonWindow,
    WindowSettings,
    Viewport,
    Glyphs,
    PressEvent,
    Button,
    Key
};
use lp::types::{SiteId, PunterId};

const CONSOLE_HEIGHT: u32 = 32;
const BORDER_WIDTH: u32 = 16;
const SCREEN_WIDTH: u32 = 640;
const SCREEN_HEIGHT: u32 = 480;

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
    InvalidPuntersCount(clap::Error),
    InvalidPunterId(clap::Error),
    InvalidTimeLimit(clap::Error),
    Piston(PistonError),
    MapFileOpen { file: String, error: io::Error, },
    MapFileDecode { file: String, error: serde_json::Error, },
    WorldNoSourceSiteId(SiteId),
    WorldNoTargetSiteId(SiteId),
    WorldNoMineSiteId(SiteId),
    WorldNoSitesAtAll,
    GNThreadSpawn(io::Error),
    GNThreadJoin(Box<std::any::Any + Send + 'static>),
    GNThreadDisconnected,
    FuturesThreadSpawn(io::Error),
    FuturesThreadJoin(Box<std::any::Any + Send + 'static>),
    FuturesThreadDisconnected,
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
        .arg(Arg::with_name("punters-count")
             .display_order(3)
             .short("c")
             .long("punters-count")
             .value_name("COUNT")
             .help("Total number of players on the map")
             .default_value("2")
             .takes_value(true))
        .arg(Arg::with_name("punter-id")
             .display_order(4)
             .short("i")
             .long("punter-id")
             .value_name("ID")
             .help("My punter id")
             .default_value("0")
             .takes_value(true))
        .arg(Arg::with_name("time-limit")
             .display_order(5)
             .short("l")
             .long("time-limit")
             .value_name("MS")
             .help("Monte-carlo time limit in ms")
             .default_value("8000")
             .takes_value(true))
        .get_matches();

    let map_file = matches.value_of("map-file")
        .ok_or(Error::MissingParameter("map-file"))?;
    let assets_dir = matches.value_of("assets-dir")
        .ok_or(Error::MissingParameter("assets-dir"))?;
    let punters_count = value_t!(matches, "punters-count", usize).map_err(Error::InvalidPuntersCount)?;
    let punter_id = value_t!(matches, "punter-id", PunterId).map_err(Error::InvalidPunterId)?;
    let time_limit_ms = value_t!(matches, "time-limit", u64).map_err(Error::InvalidTimeLimit)?;

    let opengl = OpenGL::V3_2;
    let mut window: PistonWindow = WindowSettings::new("lambda punter", [SCREEN_WIDTH, SCREEN_HEIGHT])
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

    let map = Map::new(map_file)?;
    let world = World::new(&map, punter_id, punters_count, time_limit_ms, map_file.to_string())?;

    let mut gui_state = GuiState::Standard;
    while let Some(event) = window.next() {
        window.draw_2d(&event, |context, g2d| {
            use piston_window::{clear, text, Transformed, line, Line, ellipse};
            clear([0.0, 0.0, 0.0, 1.0], g2d);
            text::Text::new_color([0.0, 1.0, 0.0, 1.0], 16).draw(
                &gui_state.console(&world),
                &mut glyphs,
                &context.draw_state,
                context.transform.trans(5.0, 20.0),
                g2d
            );

            if let Some(tr) = world.translator(&context.viewport) {
                gui_state.draw(&world, |element| match element {
                    DrawElement::River { color, radius, source_x, source_y, target_x, target_y } => {
                        line(color, radius, [tr.x(source_x), tr.y(source_y), tr.x(target_x), tr.y(target_y)], context.transform, g2d);
                    },
                    DrawElement::Mine { x, y } => {
                        ellipse([1.0, 0.0, 0.0, 1.0], [tr.x(x) - 8.0, tr.y(y) - 8.0, 16.0, 16.0], context.transform, g2d);
                    },
                    DrawElement::Future { index, color, source_x, source_y, target_x, target_y, } => {
                        text::Text::new_color(color, 24).draw(
                            &format!("{}", index),
                            &mut glyphs,
                            &context.draw_state,
                            context.transform.trans(tr.x(source_x) - 12.0, tr.y(source_y) - 12.0),
                            g2d
                        );
                        line(color, 2.0, [tr.x(target_x) - 4.0, tr.y(target_y), tr.x(target_x) + 4.0, tr.y(target_y)], context.transform, g2d);
                        line(color, 2.0, [tr.x(target_x), tr.y(target_y) - 4.0, tr.x(target_x), tr.y(target_y) + 4.0], context.transform, g2d);
                        Line::new(color, 0.5).draw_arrow(
                            [tr.x(source_x), tr.y(source_y), tr.x(target_x), tr.y(target_y)],
                            8.0, &context.draw_state, context.transform, g2d);
                    },
                });
            }
        });

        if let Some(Button::Keyboard(key)) = event.press_args() {
            gui_state = gui_state.process_key(&world, key)?;
        }
        if let GuiState::Shutdown = gui_state {
            break;
        }

        gui_state = gui_state.tick(&world)?;
    }

    Ok(())
}

#[derive(Deserialize)]
struct Site {
    id: SiteId,
    x: f64,
    y: f64,
}

#[derive(Deserialize)]
struct River {
    source: SiteId,
    target: SiteId,
}

#[derive(Deserialize)]
struct Map {
    sites: Vec<Site>,
    rivers: Vec<River>,
    mines: Vec<SiteId>,
}

impl Map {
    fn new<P>(map_file: P) -> Result<Map, Error> where P: AsRef<Path> {
        let file = io::BufReader::new(
            fs::File::open(&map_file)
                .map_err(|e| Error::MapFileOpen {
                    file: map_file.as_ref().to_string_lossy().to_string(),
                    error: e,
                })?);
        serde_json::from_reader(file).map_err(|e| Error::MapFileDecode {
            file: map_file.as_ref().to_string_lossy().to_string(),
            error: e,
        })
    }
}

struct RiverRef<'a> {
    source: &'a Site,
    target: &'a Site,
}

struct World<'a> {
    map_file: String,
    punter_id: PunterId,
    punters_count: usize,
    time_limit_ms: u64,
    rivers_refs: Vec<RiverRef<'a>>,
    mines_refs: Vec<&'a Site>,
    bounds: (f64, f64, f64, f64),
    graph: Arc<lp::graph::Graph>,
}

enum DrawElement {
    River {
        color: [f32; 4],
        radius: f64,
        source_x: f64,
        source_y: f64,
        target_x: f64,
        target_y: f64,
    },
    Mine {
        x: f64,
        y: f64,
    },
    Future {
        index: usize,
        color: [f32; 4],
        source_x: f64,
        source_y: f64,
        target_x: f64,
        target_y: f64,
    },
}

impl<'a> World<'a> {
    fn new(map: &'a Map, punter_id: PunterId, punters_count: usize, time_limit_ms: u64, map_file: String) -> Result<World<'a>, Error> {
        let mut rivers_refs = Vec::with_capacity(map.rivers.len());
        for &River { source, target, } in map.rivers.iter() {
            rivers_refs.push(RiverRef {
                source: map.sites.iter().find(|site| site.id == source).ok_or_else(|| Error::WorldNoSourceSiteId(source))?,
                target: map.sites.iter().find(|site| site.id == target).ok_or_else(|| Error::WorldNoTargetSiteId(target))?,
            });
        }

        let mut mines_refs = Vec::with_capacity(map.mines.len());
        for &site_id in map.mines.iter() {
            mines_refs.push(
                map.sites.iter().find(|site| site.id == site_id).ok_or_else(|| Error::WorldNoMineSiteId(site_id))?);
        }

        let mut bounds = None;
        for &Site { x, y, .. } in map.sites.iter() {
            if let Some((ref mut min_x, ref mut min_y, ref mut max_x, ref mut max_y)) = bounds {
                if x < *min_x { *min_x = x; }
                if y < *min_y { *min_y = y; }
                if x > *max_x { *max_x = x; }
                if y > *max_y { *max_y = y; }
            } else {
                bounds = Some((x, y, x, y));
            }
        }

        Ok(World {
            map_file: map_file,
            punter_id: punter_id,
            punters_count: punters_count,
            time_limit_ms: time_limit_ms,
            rivers_refs: rivers_refs,
            mines_refs: mines_refs,
            bounds: bounds.ok_or(Error::WorldNoSitesAtAll)?,
            graph: Arc::new(lp::graph::Graph::from_iter(map.rivers.iter().map(|r| (r.source, r.target)))),
        })
    }

    fn draw<DF>(&self, draw_element: DF) where DF: FnMut(DrawElement) {
        self.draw_custom(draw_element, |_, _| ([0.0, 0.0, 1.0, 1.0], 1.0))
    }

    fn draw_custom<DF, RF>(&self, mut draw_element: DF, river_setup: RF)
        where DF: FnMut(DrawElement),
              RF: Fn(SiteId, SiteId) -> ([f32; 4], f64),
    {
        for &RiverRef { source, target } in self.rivers_refs.iter() {
            let (color, radius) = river_setup(source.id, target.id);
            draw_element(DrawElement::River {
                color: color,
                radius: radius,
                source_x: source.x,
                source_y: source.y,
                target_x: target.x,
                target_y: target.y,
            });
        }

        for &site in self.mines_refs.iter() {
            draw_element(DrawElement::Mine { x: site.x, y: site.y, });
        }
    }

    fn translator(&self, viewport: &Option<Viewport>) -> Option<ViewportTranslator> {
        let (w, h) = viewport
            .map(|v| (v.draw_size[0], v.draw_size[1]))
            .unwrap_or((SCREEN_WIDTH, SCREEN_HEIGHT));

        if (w <= 2 * BORDER_WIDTH) || (h <= BORDER_WIDTH + CONSOLE_HEIGHT) {
            None
        } else {
            Some(ViewportTranslator {
                scale_x: (w - BORDER_WIDTH - BORDER_WIDTH) as f64 / (self.bounds.2 - self.bounds.0),
                scale_y: (h - BORDER_WIDTH - CONSOLE_HEIGHT) as f64 / (self.bounds.3 - self.bounds.1),
                min_x: self.bounds.0,
                min_y: self.bounds.1,
            })
        }
    }
}

struct ViewportTranslator {
    scale_x: f64,
    scale_y: f64,
    min_x: f64,
    min_y: f64,
}

impl ViewportTranslator {
    fn x(&self, x: f64) -> f64 {
        (x - self.min_x) * self.scale_x + BORDER_WIDTH as f64
    }

    fn y(&self, y: f64) -> f64 {
        (y - self.min_y) * self.scale_y + CONSOLE_HEIGHT as f64
    }
}

enum GuiState {
    Standard,
    GirvanNewmanInProgress {
        slave: thread::JoinHandle<()>,
        rx: mpsc::Receiver<(HashMap<lp::map::River, f64>, Option<(f64, f64)>)>,
    },
    GirvanNewman {
        gn_table: HashMap<lp::map::River, f64>,
        gn_bounds: (f64, f64),
    },
    FuturesInProgress {
        slave: thread::JoinHandle<()>,
        rx: mpsc::Receiver<Vec<(SiteId, SiteId)>>,
    },
    Futures {
        futures: Vec<(f64, f64, f64, f64)>,
    },
    Shutdown,
}

impl GuiState {
    fn console<'a>(&self, world: &World<'a>) -> String {
        match self {
            &GuiState::Standard =>
                format!("Map [ {} ]. Press <G> to calculate Girvan-Newman or <F> to declare futures.", world.map_file),
            &GuiState::GirvanNewmanInProgress { .. } =>
                "Calculating Girvan-Newman coeffs, please wait...".to_string(),
            &GuiState::GirvanNewman { .. } =>
                "Girvan-Newmap coeffs visualizer. Press <S> to return.".to_string(),
            &GuiState::FuturesInProgress { .. } =>
                "Estimating best futures, please wait...".to_string(),
            &GuiState::Futures { ref futures, } =>
                format!("Declared {} futures. Press <S> to return.", futures.len()),
            &GuiState::Shutdown =>
                "Shutting down...".to_string(),
        }
    }

    fn draw<'a, DF>(&self, world: &World<'a>, mut draw_element: DF)
        where DF: FnMut(DrawElement)
    {
        match self {
            &GuiState::Standard | &GuiState::GirvanNewmanInProgress { .. } | &GuiState::FuturesInProgress { .. } =>
                world.draw(draw_element),
            &GuiState::GirvanNewman { ref gn_table, gn_bounds: (min_c, max_c), } =>
                world.draw_custom(draw_element, |source_id, target_id| {
                    let river = lp::map::River::new(source_id, target_id);
                    if let Some(coeff) = gn_table.get(&river) {
                        let factor = (coeff - min_c) / (max_c - min_c);
                        let r = 0.5 * factor + 0.5;
                        let b = 1.0 * factor;
                        let radius = 2.5 * factor + 0.5;
                        ([r as f32, b as f32, 0.0, 1.0], radius)
                    } else {
                        ([1.0, 0.0, 0.0, 1.0], 2.0)
                    }
                }),
            &GuiState::Futures { ref futures, } => {
                world.draw(&mut draw_element);
                let colors = &[[1.0, 1.0, 1.0, 1.0],
                               [1.0, 1.0, 0.0, 1.0],
                               [1.0, 0.0, 1.0, 1.0],
                               [0.0, 1.0, 1.0, 1.0]];
                for (i, &(source_x, source_y, target_x, target_y)) in futures.iter().enumerate() {
                    draw_element(DrawElement::Future {
                        index: i,
                        color: colors[i % colors.len()],
                        source_x: source_x,
                        source_y: source_y,
                        target_x: target_x,
                        target_y: target_y,
                    });
                }
            },
            &GuiState::Shutdown =>
                (),
        }
    }

    fn process_key<'a>(self, world: &World<'a>, key: Key) -> Result<GuiState, Error> {
        Ok(match (self, key) {
            (GuiState::Standard, Key::G) => {
                let graph = world.graph.clone();
                let (tx, rx) = mpsc::channel();
                let slave = thread::Builder::new()
                    .name("girvan-newman calculator slave".to_string())
                    .spawn(move || {
                        let gn_table = graph.rivers_betweenness::<()>(&mut Default::default());
                        let mut bounds = None;
                        for &coeff in gn_table.values() {
                            if let Some((ref mut min_c, ref mut max_c)) = bounds {
                                if coeff < *min_c { *min_c = coeff; }
                                if coeff > *max_c { *max_c = coeff; }
                            } else {
                                bounds = Some((coeff, coeff));
                            }
                        }
                        tx.send((gn_table, bounds)).ok();
                    })
                    .map_err(Error::GNThreadSpawn)?;
                GuiState::GirvanNewmanInProgress { slave: slave, rx: rx, }
            },
            (GuiState::Standard, Key::F) => {
                let graph = world.graph.clone();
                let mines: Vec<_> = world.mines_refs.iter().map(|m| m.id).collect();
                let rivers_count = world.rivers_refs.len();
                let punter_id = world.punter_id;
                let punters_count = world.punters_count;
                let max_timeout = std::time::Duration::from_millis(world.time_limit_ms);

                let (tx, rx) = mpsc::channel();
                let slave = thread::Builder::new()
                    .name("futures estimator slave".to_string())
                    .spawn(move || {
                        let mut gcache = Default::default();
                        let mut mcache = Default::default();
                        let gn_table = lp::map::RiversIndex::from_hash_map(
                            graph.rivers_betweenness(&mut gcache));
                        let mut futures = Vec::new();
                        let mut start_turn = 0;
                        let timeout_start = std::time::Instant::now();
                        for &mine in mines.iter() {
                            if let Some(time_avail) = max_timeout.checked_sub(timeout_start.elapsed()) {
                                let maybe_future = lp::prob::estimate_best_future(
                                    &graph,
                                    mine,
                                    &mines,
                                    &gn_table,
                                    punter_id,
                                    punters_count,
                                    start_turn,
                                    |path_rivers, claimed_rivers| {
                                        path_rivers
                                            .iter()
                                            .filter(|&r| !claimed_rivers.contains_key(r))
                                            .max_by_key(|&r| gn_table.get(r).map(|bw| (bw * 1000.0) as u64).unwrap_or(0))
                                    },
                                    std::cmp::min(std::cmp::max(rivers_count, 128), 1024),
                                    time_avail,
                                    &mut mcache,
                                    &mut gcache);
                                if let Some((source, target, path_len)) = maybe_future {
                                    futures.push((source, target));
                                    start_turn += path_len * punters_count;
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        tx.send(futures).ok();
                    })
                    .map_err(Error::FuturesThreadSpawn)?;
                GuiState::FuturesInProgress { slave: slave, rx: rx, }
            },
            (GuiState::GirvanNewman { .. }, Key::S) =>
                GuiState::Standard,
            (GuiState::Futures { .. }, Key::S) =>
                GuiState::Standard,
            (_, Key::Q) =>
                GuiState::Shutdown,
            (other, _) =>
                other,
        })
    }

    fn tick<'a>(self, world: &World<'a>) -> Result<Self, Error> {
        match self {
            GuiState::GirvanNewmanInProgress { slave, rx, } =>
                match rx.try_recv() {
                    Ok((gn_table, bounds)) => {
                        let () = slave.join().map_err(Error::GNThreadJoin)?;
                        Ok(GuiState::GirvanNewman { gn_table: gn_table, gn_bounds: bounds.unwrap_or((1.0, 1.0)), })
                    },
                    Err(mpsc::TryRecvError::Empty) =>
                        Ok(GuiState::GirvanNewmanInProgress { slave: slave, rx: rx, }),
                    Err(mpsc::TryRecvError::Disconnected) =>
                        Err(Error::GNThreadDisconnected),
                },
            GuiState::FuturesInProgress { slave, rx, } =>
                match rx.try_recv() {
                    Ok(raw_futures) => {
                        let find_ref = |site_id| {
                            for &RiverRef { source, target } in world.rivers_refs.iter() {
                                if source.id == site_id {
                                    return Some(source);
                                }
                                if target.id == site_id {
                                    return Some(target);
                                }
                            }
                            None
                        };
                        let mut futures = Vec::with_capacity(raw_futures.len());
                        for (source, target) in raw_futures {
                            if let (Some(source_ref), Some(target_ref)) = (find_ref(source), find_ref(target)) {
                                futures.push((source_ref.x, source_ref.y, target_ref.x, target_ref.y));
                            }
                        }
                        let () = slave.join().map_err(Error::FuturesThreadJoin)?;
                        Ok(GuiState::Futures { futures: futures, })
                    },
                    Err(mpsc::TryRecvError::Empty) =>
                        Ok(GuiState::FuturesInProgress { slave: slave, rx: rx, }),
                    Err(mpsc::TryRecvError::Disconnected) =>
                        Err(Error::FuturesThreadDisconnected),
                },
            other =>
                Ok(other),
        }
    }
}
