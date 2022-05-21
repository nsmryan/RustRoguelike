use std::io::BufRead;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct AtlasSheet {
    pub name: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl AtlasSheet {
    pub fn new( name: String, x: u32, y: u32, width: u32, height: u32) -> AtlasSheet {
        return AtlasSheet { name, x, y, width, height };
    }
}

impl FromStr for AtlasSheet {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let s: &mut str = &mut string.to_string();

        let mut args = s.split(" ");

        let name = args.next().unwrap().to_string();
        let x = args.next().ok_or("no arg")?.parse::<u32>().map_err(|err| format!("{}", err))?;
        let y = args.next().ok_or("no arg")?.parse::<u32>().map_err(|err| format!("{}", err))?;
        let width = args.next().ok_or("no arg")?.parse::<u32>().map_err(|err| format!("{}", err))?;
        let height = args.next().ok_or("no arg")?.parse::<u32>().map_err(|err| format!("{}", err))?;
        
        return Ok(AtlasSheet::new(name, x, y, width, height));
    }
}

pub struct AtlasFont {
    pub name: String,
    pub coords: Vec<(u32, u32)>,
    pub width: u32,
    pub height: u32,
}

pub struct Atlas {
    sheets: Vec<AtlasSheet>,
    fonts: HashMap<String, AtlasFont>,
}

pub fn parse_atlas_file(atlas_file: &str) -> Vec<AtlasSheet> {
    let file =
        std::fs::File::open(&atlas_file).expect(&format!("Could not open atlas file '{}'", atlas_file));

    let mut sheets: Vec<AtlasSheet> = Vec::new();
    for line in std::io::BufReader::new(file).lines() {
        if let Ok(sheet) = AtlasSheet::from_str(&line.unwrap()) { 
            sheets.push(sheet);
        }
    }

    return sheets;
}

