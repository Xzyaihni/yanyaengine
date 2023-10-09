use nalgebra::{
    Vector3,
    Point3,
    base::Matrix4
};


pub struct CameraTransformConfig
{
    pub position: Point3<f32>,
    pub forward: Vector3<f32>
}

impl Default for CameraTransformConfig
{
    fn default() -> Self
    {
        Self{
            position: Point3::new(0.0, 0.0, 0.0),
            forward: Vector3::z()
        }
    }
}

#[derive(Debug, Clone)]
pub struct CameraTransform
{
    position: Point3<f32>,
    forward: Vector3<f32>,
    up: Vector3<f32>,
    right: Vector3<f32>,
    matrix: Matrix4<f32>
}

#[allow(dead_code)]
impl CameraTransform
{
    pub fn new(config: CameraTransformConfig) -> Self
    {
        let right = Self::calculate_right(&config.forward);
        let up = Self::calculate_up(&config.forward, &right);

        let matrix = Self::calculate_matrix(&config.position, &config.forward, &up);

        Self{
            position: config.position,
            forward: config.forward,
            up,
            right,
            matrix
        }
    }

    fn calculate_right(forward: &Vector3<f32>) -> Vector3<f32>
    {
        let global_up = Vector3::y();

        global_up.cross(forward).normalize()
    }

    fn calculate_up(forward: &Vector3<f32>, right: &Vector3<f32>) -> Vector3<f32>
    {
        forward.cross(right).normalize()
    }

    pub fn set_position(&mut self, position: Point3<f32>)
    {
        self.position = position;
    }

    pub fn translate(&mut self, translation: Vector3<f32>)
    {
        self.position += translation;
    }

    pub fn recalculate_matrix(&mut self)
    {
        self.right = Self::calculate_right(&self.forward);
        self.up = Self::calculate_up(&self.forward, &self.right);

        self.matrix = Self::calculate_matrix(&self.position, &self.forward, &self.up);
    }

    fn calculate_matrix(
        position: &Point3<f32>,
        forward: &Vector3<f32>,
        up: &Vector3<f32>
    ) -> Matrix4<f32>
    {
        let target = *position + forward;

        Matrix4::look_at_rh(position, &target, up)
    }

    pub fn matrix(&self) -> Matrix4<f32>
    {
        self.matrix
    }
}
