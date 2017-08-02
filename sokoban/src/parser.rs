use nom::{self, IResult};
use super::map::{Coords, Tile, Room};
use super::game::{Game, GameState};

#[derive(PartialEq)]
pub enum DataElement {
    Wall,
    Floor,
    Player,
    Crate,
    CrateDst,
    CrateOnDst,
}

named!(wall<&[u8], DataElement>, map!(char!('#'), |_| DataElement::Wall));
named!(floor<&[u8], DataElement>, map!(char!(' '), |_| DataElement::Floor));
named!(player<&[u8], DataElement>, map!(char!('I'), |_| DataElement::Player));
named!(crate_<&[u8], DataElement>, map!(char!('+'), |_| DataElement::Crate));
named!(crate_dst<&[u8], DataElement>, map!(char!('@'), |_| DataElement::CrateDst));
named!(crate_on_dst<&[u8], DataElement>, map!(char!('*'), |_| DataElement::CrateOnDst));
named!(dataline<&[u8], Vec<DataElement>>, many0!(alt!(wall | floor | player | crate_ | crate_dst | crate_on_dst)));
named!(roomdef<Vec<Vec<DataElement>>>, separated_list_complete!(alt!(char!('\r') | char!('\n')), dataline));

#[derive(Debug)]
pub enum Error {
    RoomIsEmpty,
    WidthMismatch { min: usize, max: usize, },
    ParseNom(nom::ErrorKind),
    ParseIncomplete,
    NoCratesInRoom,
    CratesAndDestinationsMismatch { crates_count: usize, crates_dst_count: usize, },
    InvalidPlayerStartPositionsCount(usize),
}

fn width_from(rd: &Vec<Vec<DataElement>>) -> Result<usize, Error> {
    let min_width = rd.iter().map(|l| l.len()).min();
    let max_width = rd.iter().map(|l| l.len()).max();

    debug!("width_from: min_width = {:?}, max_width = {:?}", min_width, max_width);
    match (min_width, max_width) {
        (Some(min), Some(max)) if min == max =>
            Ok(min),
        (Some(min), Some(max)) =>
            Err(Error::WidthMismatch { min: min, max: max, }),
        (None, _) | (_, None) =>
            Err(Error::RoomIsEmpty),
    }
}

fn make_room(width: usize, height: usize, rd: &Vec<Vec<DataElement>>) -> Result<Room, Error> {
    let el_count = |el| rd
        .iter()
        .flat_map(|l| l.iter().filter(|e| e == &&el))
        .count();
    let crates_count = el_count(DataElement::Crate) + el_count(DataElement::CrateOnDst);
    let crates_dst_count = el_count(DataElement::CrateDst) + el_count(DataElement::CrateOnDst);
    let start_pos_count = el_count(DataElement::Player);
    if crates_count == 0 {
        Err(Error::NoCratesInRoom)
    } else if crates_count != crates_dst_count {
        Err(Error::CratesAndDestinationsMismatch {
            crates_count: crates_count,
            crates_dst_count: crates_dst_count,
        })
    } else if start_pos_count != 1 {
        Err(Error::InvalidPlayerStartPositionsCount(start_pos_count))
    } else {
        Ok(Room {
            width: width,
            height: height,
            crates_dsts: rd
                .iter()
                .enumerate()
                .flat_map(move |(row, data_line)| data_line.iter().enumerate().map(move |v| (row, v)))
                .filter(|&(_, (_, e))| e == &DataElement::CrateDst || e == &DataElement::CrateOnDst)
                .map(|(row, (col, _))| (row as isize, col as isize))
                .collect(),
            content: rd
                .iter()
                .flat_map(|data_line| data_line.iter().map(|e| match e {
                    &DataElement::Wall => Tile::Wall,
                    &DataElement::Floor => Tile::Floor,
                    &DataElement::CrateDst => Tile::CrateDst,
                    &DataElement::CrateOnDst => Tile::CrateDst,
                    _ => Tile::Floor,
                }))
                .collect(),
        })
    }
}

fn make_init_state(game: &mut Game, width: usize, rd: &Vec<Vec<DataElement>>) -> Result<GameState, Error> {
    let coords_of = |els: &[DataElement]| -> Vec<Coords> {
        rd
            .iter()
            .flat_map(|l| l.iter())
            .enumerate()
            .filter(move |&(_, ref e)| els.iter().any(|el| el == *e))
            .map(|(coord, _)| ((coord / width) as isize, (coord % width) as isize)).collect()
    };
    let player_coords = coords_of(&[DataElement::Player]);

    if player_coords.len() == 1 {
        let player = player_coords[0];
        let crates: Vec<_> = coords_of(&[DataElement::Crate, DataElement::CrateOnDst]);    
        let placement = game.make_placement(player, &crates);
        
        Ok(game.make_game_state(placement))
    } else {
        Err(Error::InvalidPlayerStartPositionsCount(0))
    }
}

pub fn parse(input: &[u8]) -> Result<(Game, GameState), Error> {
    let rd: Vec<Vec<DataElement> >  = match roomdef(input) {
        IResult::Done(_, rdb) =>
            rdb,
        IResult::Error(e) =>
            return Err(Error::ParseNom(e)),
        IResult::Incomplete(_) =>
            return Err(Error::ParseIncomplete),
    };
    let width = width_from(&rd)?;
    let height = rd.len();
    let room = make_room(width, height, &rd)?;

    let mut game = Game::new(room);
    let init_state = make_init_state(&mut game, width, &rd)?;
    Ok((game, init_state))
}
