use std::f32;

use nalgebra::{
    Point3,
    Vector2,
    Vector3,
    geometry::Orthographic3,
    Matrix4
};

use camera_transform::CameraTransform;

mod camera_transform;


#[derive(Debug, Clone)]
pub struct Camera
{
    projection: Matrix4<f32>,
    view: CameraTransform,
    projection_view: Matrix4<f32>,
    aspect: f32,
    scale: f32,
    size: Vector2<f32>,
    z_height: f32
}

impl Camera
{
    pub fn new(aspect: f32, z_height: f32) -> Self
    {
        let size = Self::aspect_size(aspect);
        let projection = Self::create_projection(size, z_height);

        let view = CameraTransform::new(Default::default());

        let projection_view = Self::create_projection_view(projection, view.matrix());

        Self{
            projection,
            view,
            projection_view,
            aspect,
            scale: 1.0,
            size,
            z_height
        }
    }

    fn aspect_size(aspect: f32) -> Vector2<f32>
    {
        if aspect < 1.0
        {
            Vector2::new(aspect, 1.0)
        } else
        {
            Vector2::new(1.0, aspect.recip())
        }
    }

    fn create_projection(size: Vector2<f32>, z_height: f32) -> Matrix4<f32>
    {
        let identity = Matrix4::identity();
        let mut projection = Orthographic3::from_matrix_unchecked(identity);

        let size = size / 2.0;
        projection.set_left_and_right(-size.x, size.x);
        projection.set_bottom_and_top(-size.y, size.y);

        projection.set_znear_and_zfar(-z_height, z_height);

        projection.to_homogeneous()
    }

    fn recreate_projection(&mut self, size: Vector2<f32>)
    {
        self.size = size;

        self.projection = Self::create_projection(self.size, self.z_height);

        self.regenerate_projection_view();
    }

    pub fn update(&mut self)
    {
        self.view.update();

        self.regenerate_projection_view();
    }

    pub fn screen_position(&self, position: Vector2<f32>) -> Vector2<f32>
    {
        let offset = position - self.position().coords.xy();

        offset.component_div(&self.size())
    }

    pub fn screen_size(&self, size: Vector2<f32>) -> Vector2<f32>
    {
        size.component_div(&self.size())
    }

    pub fn position(&self) -> &Point3<f32>
    {
        self.view.position()
    }

    pub fn set_position(&mut self, position: Point3<f32>)
    {
        self.view.set_position(position);
    }

    pub fn set_position_x(&mut self, position: f32)
    {
        self.view.set_position_x(position);
    }

    pub fn set_position_y(&mut self, position: f32)
    {
        self.view.set_position_y(position);
    }

    pub fn set_position_z(&mut self, position: f32)
    {
        self.view.set_position_z(position);
    }

    pub fn translate(&mut self, translation: Vector3<f32>)
    {
        self.view.translate(translation);
    }

    pub fn translate_to(&mut self, other: &Vector3<f32>, amount: f32)
    {
        self.view.translate_to(other, amount);
    }

    fn regenerate_projection_view(&mut self)
    {
        self.projection_view =
            Self::create_projection_view(self.projection, self.view.matrix());
    }

    pub fn create_projection_view(projection: Matrix4<f32>, view: Matrix4<f32>) -> Matrix4<f32>
    {
        projection * view
    }

    pub fn projection_view(&self) -> Matrix4<f32>
    {
        self.projection_view
    }

    pub fn resize(&mut self, aspect: f32)
    {
        self.aspect = aspect;

        //this one just changes the aspect ratio
        self.recreate_projection(Self::aspect_size(aspect) * self.scale);
    }

    pub fn rescale(&mut self, scale: f32)
    {
        self.scale = scale;

        //this one actually scales the view
        self.resize(self.aspect);
    }

    pub fn aspect(&self) -> f32
    {
        self.size.x / self.size.y
    }

    pub fn size(&self) -> Vector2<f32>
    {
        self.size
    }

    pub fn size3d(&self) -> Vector3<f32>
    {
        Vector3::new(self.size.x, self.size.y, self.z_height)
    }

    pub fn over_size(&self) -> Vector2<f32>
    {
        let lowest = self.size.x.min(self.size.y);

        self.size / lowest
    }

    pub fn normalized_size(&self) -> Vector2<f32>
    {
        let highest = self.size.x.max(self.size.y);

        self.size / highest
    }
}
