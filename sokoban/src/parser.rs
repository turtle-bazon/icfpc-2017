use map;
use std::vec;
use nom::IResult;

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
named!(dataline<&[u8], Vec<DataElement> >, many0!(alt!(wall | floor | player | cube | destination)));
named!(roomdef<Vec<Vec<DataElement> > >, separated_list_complete!(alt!(char!('\r') | char!('\n')), dataline));

fn width_from(rd: &Vec<Vec <DataElement> >) -> usize {
    let mut max_width: usize = 0;
    let mut min_width: usize = 65535;

    for data_line in rd.iter() {
        let current_width = data_line.len();

        if (current_width > max_width) {
            max_width = current_width;
        }

        if (current_width < min_width) {
            min_width = current_width;
        }
    }

    if (max_width != min_width) {
        panic!("Error");
    }
    
    max_width
}

fn content_from(width: usize, height: usize, rd: &Vec<Vec <DataElement> >) -> Vec<map::Map> {
    let mut content: Vec<map::Map> = Vec::new();

    for data_line in rd.iter() {
        for data_element in data_line.iter() {
            content.push(match *data_element {
                DataElement::Wall => map::Map::Wall,
                DataElement::Floor => map::Map::Floor,
                _ => map::Map::Floor,
            });
        }
    }

    content
}

pub fn parse_map(input: &[u8]) -> map::Room {
    let rd: Vec<Vec<DataElement> >  = match roomdef(input) {
        IResult::Done(_, rdb) => rdb,
        IResult::Error(_) => panic!("error"),
        IResult::Incomplete(_) => panic!("incomplete"),
    };
    let width = width_from (&rd);
    let height = rd.len();

    map::Room{
        width: width,
        height: height,
        content: content_from(width, height, &rd),
    }
}
