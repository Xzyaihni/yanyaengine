use std::path::Path;

use serde::{Serialize, Deserialize};

use nalgebra::Vector3;

use strum::EnumIter;


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

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, EnumIter, bincode::Decode, bincode::Encode)]
pub enum Uvs
{
    Normal,
    FlipHorizontal,
    FlipVertical,
    FlipBoth
}

impl Default for Uvs
{
    fn default() -> Self
    {
        Self::Normal
    }
}

impl Uvs
{
    fn bottom_left(&self) -> [f32; 2]
    {
        self.remap([0.0, 0.0])
    }

    fn bottom_right(&self) -> [f32; 2]
    {
        self.remap([1.0, 0.0])
    }

    fn top_left(&self) -> [f32; 2]
    {
        self.remap([0.0, 1.0])
    }

    fn top_right(&self) -> [f32; 2]
    {
        self.remap([1.0, 1.0])
    }

    fn remap(&self, uvs: [f32; 2]) -> [f32; 2]
    {
        match self
        {
            Self::Normal => uvs,
            Self::FlipHorizontal => [1.0 - uvs[0], uvs[1]],
            Self::FlipVertical => [uvs[0], 1.0 - uvs[1]],
            Self::FlipBoth => [1.0 - uvs[0], 1.0 - uvs[1]]
        }
    }
}

#[derive(Debug, Clone)]
pub struct Model
{
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<u16>,
    pub uvs: Vec<[f32; 2]>
}

#[allow(dead_code)]
impl Model
{
    pub fn new() -> Self
    {
        Self{vertices: Vec::new(), indices: Vec::new(), uvs: Vec::new()}
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ParseError>
    {
        let parser = ObjParser::new();

        parser.parse(path)
    }

    pub fn square(side: f32) -> Self
    {
        Self::square_with_uvs(Uvs::Normal, side)
    }

    pub fn square_with_uvs(uvs: Uvs, side: f32) -> Self
    {
        Self::rectangle_with_uvs(uvs, side, side)
    }

    pub fn rectangle(width: f32, height: f32) -> Self
    {
        Self::rectangle_with_uvs(Uvs::Normal, width, height)
    }

    pub fn rectangle_with_uvs(uvs: Uvs, width: f32, height: f32) -> Self
    {
        let (half_width, half_height) = (width / 2.0, height / 2.0);

        let vertices = vec![
            [-half_width, -half_height, 0.0],
            [-half_width, half_height, 0.0],
            [half_width, -half_height, 0.0],
            [half_width, half_height, 0.0]
        ];

        let indices = vec![0, 1, 2, 1, 3, 2];

        let uvs = vec![
            uvs.bottom_left(),
            uvs.top_left(),
            uvs.bottom_right(),
            uvs.top_right()
        ];

        Self{vertices, indices, uvs}
    }

    pub fn shift(&mut self, offset: Vector3<f32>)
    {
        self.vertices.iter_mut().for_each(|vertex|
        {
            vertex[0] += offset.x;
            vertex[1] += offset.y;
            vertex[2] += offset.z;
        });
    }
}

struct ObjParser
{
    vertices: Vec<[f32; 3]>,
    indices: Vec<u16>,
    uvs: Vec<[f32; 2]>
}

impl ObjParser
{
    pub fn new() -> Self
    {
        let vertices = Vec::new();
        let indices = Vec::new();
        let uvs = Vec::new();

        Self{vertices, indices, uvs}
    }

    pub fn parse<P: AsRef<Path>>(self, _path: P) -> Result<Model, ParseError>
    {
        // ill do this later wutever blablabla

        Ok(Model{vertices: self.vertices, indices: self.indices, uvs: self.uvs})
    }
}
