use nom::IResult;
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

fn width_from(rd: &Vec<Vec<DataElement>>) -> usize {
    let mut max_width: usize = 0;
    let mut min_width: usize = 65535;

    for data_line in rd.iter() {
        let current_width = data_line.len();

        if current_width > max_width {
            max_width = current_width;
        }

        if current_width < min_width {
            min_width = current_width;
        }
    }

    if max_width != min_width {
        panic!("Error");
    }

    max_width
}

fn content_from(width: usize, height: usize, rd: Vec<Vec<DataElement>>) -> Room {
    Room {
        width: width,
        height: height,
        content: rd
            .into_iter()
            .flat_map(|data_line| data_line.into_iter().map(|e| match e {
                DataElement::Wall => Tile::Wall,
                DataElement::Floor => Tile::Floor,
                _ => Tile::Floor,
            }))
            .collect(),
    }
}

pub fn parse_map(input: &[u8]) -> Room {
    let rd: Vec<Vec<DataElement> >  = match roomdef(input) {
        IResult::Done(_, rdb) => rdb,
        IResult::Error(_) => panic!("error"),
        IResult::Incomplete(_) => panic!("incomplete"),
    };
    let width = width_from (&rd);
    let height = rd.len();

    content_from(width, height, rd)
}
