use serde::{Serialize, Deserialize};

use nalgebra::{
	Vector2,
	Vector3
};


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, bincode::Decode, bincode::Encode)]
pub struct Transform
{
	pub rotation: f32,
    #[bincode(with_serde)]
	pub position: Vector3<f32>,
    #[bincode(with_serde)]
	pub scale: Vector3<f32>,
    #[bincode(with_serde)]
	pub stretch: (f32, Vector2<f32>)
}

impl Default for Transform
{
    fn default() -> Self
    {
		let rotation = 0.0;

		let position = Vector3::zeros();
		let scale = Vector3::repeat(1.0);

		let stretch = (0.0, Vector2::repeat(1.0));

		Self{rotation, position, scale, stretch}
    }
}

impl Transform
{
	pub fn new() -> Self
	{
        Self::default()
	}

	pub fn half(&self) -> Vector3<f32>
	{
		self.scale / 2.0
	}

	pub fn distance(&self, value: Vector3<f32>) -> f32
	{
		Self::distance_associated(self.position, value)
	}

	pub fn direction(&self, value: Vector3<f32>) -> Vector3<f32>
	{
		value - self.position
	}

	pub fn interpolate(value0: f32, value1: f32, amount: f32) -> f32
	{
		value0 * (1.0 - amount) + value1 * amount
	}

	pub fn interpolate_vector(
		value0: Vector3<f32>,
		value1: Vector3<f32>,
		amount: f32
	) -> Vector3<f32>
	{
		Vector3::new(
			Self::interpolate(value0.x, value1.x, amount),
			Self::interpolate(value0.y, value1.y, amount),
			Self::interpolate(value0.z, value1.z, amount)
		)
	}

	pub fn distance_associated(value0: Vector3<f32>, value1: Vector3<f32>) -> f32
	{
		(value1 - value0).magnitude()
	}

    pub fn max_scale(&self) -> f32
    {
        let scale = self.scale;

        scale.x.max(scale.y.max(scale.z))
    }
}

pub trait OnTransformCallback
{
	fn callback(&mut self) {}

	fn transform_callback(&mut self, _transform: Transform)
	{
		self.callback();
	}

	fn position_callback(&mut self, _position: Vector3<f32>)
	{
		self.callback();
	}

	fn scale_callback(&mut self, _scale: Vector3<f32>)
	{
		self.callback();
	}

	fn rotation_callback(&mut self, _rotation: f32)
	{
		self.callback();
	}

	fn stretch_callback(&mut self, _stretch: (f32, Vector2<f32>))
	{
		self.callback();
	}
}

#[allow(dead_code)]
pub trait TransformContainer: OnTransformCallback
{
	fn transform_ref(&self) -> &Transform;
	fn transform_mut(&mut self) -> &mut Transform;

	fn transform_clone(&self) -> Transform
	{
		self.transform_ref().clone()
	}

	fn set_transform(&mut self, transform: Transform)
	{
		self.set_transform_only(transform.clone());
		self.transform_callback(transform);
	}

	fn set_transform_only(&mut self, transform: Transform)
	{
		*self.transform_mut() = transform;
	}

	fn position(&self) -> &Vector3<f32>
	{
		&self.transform_ref().position
	}

	fn interpolate_position(&self, value: Vector3<f32>, amount: f32) -> Vector3<f32>
	{
		Transform::interpolate_vector(self.transform_ref().position, value, amount)
	}

	fn translate_to(&mut self, value: Vector3<f32>, amount: f32)
	{
		let new_position = self.interpolate_position(value, amount);

		self.set_position(new_position);
	}

	fn distance(&self, value: Vector3<f32>) -> f32
	{
		self.transform_ref().distance(value)
	}

	fn direction(&self, value: Vector3<f32>) -> Vector3<f32>
	{
		self.transform_ref().direction(value)
	}

	fn translate(&mut self, position: Vector3<f32>)
	{
		self.set_position(self.position() + position);
	}

	fn set_position(&mut self, position: Vector3<f32>)
	{
		self.transform_mut().position = position;
		self.position_callback(position);
	}

	fn set_position_x(&mut self, x: f32)
	{
		self.transform_mut().position.x = x;
		self.position_callback(self.transform_ref().position);
	}

	fn set_position_y(&mut self, y: f32)
	{
		self.transform_mut().position.y = y;
		self.position_callback(self.transform_ref().position);
	}

	fn set_position_z(&mut self, z: f32)
	{
		self.transform_mut().position.z = z;
		self.position_callback(self.transform_ref().position);
	}

	fn scale(&self) -> &Vector3<f32>
	{
		&self.transform_ref().scale
	}

	fn set_scale(&mut self, scale: Vector3<f32>)
	{
		self.transform_mut().scale = scale;
		self.scale_callback(scale);
	}

	fn grow(&mut self, scale: Vector3<f32>)
	{
		self.set_scale(self.scale() + scale);
	}

	fn rotation(&self) -> f32
	{
		self.transform_ref().rotation
	}

	fn set_rotation(&mut self, rotation: f32)
	{
		self.transform_mut().rotation = rotation;
		self.rotation_callback(rotation);
	}

	fn half(&self) -> Vector3<f32>
	{
		self.transform_ref().half()
	}

	fn set_stretch(&mut self, stretch: (f32, Vector2<f32>))
	{
		self.transform_mut().stretch = stretch;
		self.stretch_callback(stretch);
	}
}
