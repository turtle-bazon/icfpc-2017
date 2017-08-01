use nom::{self, IResult};
use super::map::{Tile, Room};

#[derive (Debug)]
pub enum DataElement {
    Wall,
    Floor,
    Player,
    Cube,
    Destination,
}

named!(wall<&[u8], DataElement>, map!(char!('#'), |_| DataElement::Wall));
named!(floor<&[u8], DataElement>, map!(char!(' '), |_| DataElement::Floor));
named!(player<&[u8], DataElement>, map!(char!('I'), |_| DataElement::Player));
named!(cube<&[u8], DataElement>, map!(char!('+'), |_| DataElement::Cube));
named!(destination<&[u8], DataElement>, map!(char!('@'), |_| DataElement::Destination));
named!(dataline<&[u8], Vec<DataElement>>, many0!(alt!(wall | floor | player | cube | destination)));
named!(roomdef<Vec<Vec<DataElement>>>, separated_list_complete!(alt!(char!('\r') | char!('\n')), dataline));

#[derive(Debug)]
pub enum Error {
    RoomIsEmpty,
    WidthMismatch { min: usize, max: usize, },
    ParseNom(nom::ErrorKind),
    ParseIncomplete,
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

fn room_from(width: usize, height: usize, rd: &Vec<Vec<DataElement>>) -> Room {
    Room {
        width: width,
        height: height,
        content: rd
            .iter()
            .flat_map(|data_line| data_line.iter().map(|e| match e {
                &DataElement::Wall => Tile::Wall,
                &DataElement::Floor => Tile::Floor,
                _ => Tile::Floor,
            }))
            .collect(),
    }
}

pub fn parse_map(input: &[u8]) -> Result<Room, Error> {
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

    Ok(room_from(width, height, &rd))
}
