use std::path::Path;


type LineNumber = u32;

#[allow(dead_code)]
#[derive(Debug)]
pub struct ParseError
{
    line_number: LineNumber,
    kind: ParseErrorKind
}

#[derive(Debug)]
pub enum ParseErrorKind
{
    
}

#[derive(Debug)]
pub struct Model
{
    pub vertices: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>
}

#[allow(dead_code)]
impl Model
{
    pub fn new() -> Self
    {
        Self{vertices: Vec::new(), uvs: Vec::new()}
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ParseError>
    {
        let parser = ObjParser::new();

        parser.parse(path)
    }

    pub fn square(side: f32) -> Self
    {
        Self::rectangle(side, side)
    }

    pub fn rectangle(width: f32, height: f32) -> Self
    {
        let (half_width, half_height) = (width / 2.0, height / 2.0);

        let vertices = vec![
            [-half_width, -half_height, 0.0],
            [-half_width, half_height, 0.0],
            [half_width, -half_height, 0.0],
            [-half_width, half_height, 0.0],
            [half_width, half_height, 0.0],
            [half_width, -half_height, 0.0]
        ];

        let uvs = vec![
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0]
        ];

        Self{vertices, uvs}
    }
}

struct ObjParser
{
    vertices: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>
}

impl ObjParser
{
    pub fn new() -> Self
    {
        let vertices = Vec::new();
        let uvs = Vec::new();

        Self{vertices, uvs}
    }

    pub fn parse<P: AsRef<Path>>(self, _path: P) -> Result<Model, ParseError>
    {
        // ill do this later wutever blablabla

        Ok(Model{vertices: self.vertices, uvs: self.uvs})
    }
}
