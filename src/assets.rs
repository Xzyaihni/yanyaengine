use std::{
    fs,
    fmt,
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc
};

use parking_lot::RwLock;

use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::{
    PipelineInfo,
    object::{
        resource_uploader::ResourceUploader,
        model::Model,
        texture::{RgbaImage, Texture}
    }
};


#[derive(EnumIter)]
pub enum DefaultModel
{
    Square
}

pub struct NamedValue<T>
{
    pub name: String,
    pub value: T
}

impl<T> NamedValue<T>
{
    pub fn map<U, F>(self, f: F) -> NamedValue<U>
    where
        F: FnOnce(T) -> U
    {
        let NamedValue{
            name,
            value
        } = self;

        NamedValue{
            name,
            value: f(value)
        }
    }
}

pub struct FilesLoader;

impl FilesLoader
{
    pub fn load_images(folder_path: impl AsRef<Path>) -> impl Iterator<Item=NamedValue<RgbaImage>>
    {
        Self::load(folder_path).map(|named_value|
        {
            named_value.map(|path| RgbaImage::load(path).unwrap())
        })
    }

    pub fn load(folder_path: impl AsRef<Path>) -> impl Iterator<Item=NamedValue<PathBuf>>
    {
		Self::recursive_dir(folder_path.as_ref()).map(move |name|
		{
            let value = name.clone();

			let short_path = name.strip_prefix(folder_path.as_ref())
                .expect("all paths must be in parent folder")
                .to_string_lossy().into_owned();

            NamedValue{name: short_path, value}
		})
    }

	fn recursive_dir(path: &Path) -> impl Iterator<Item=PathBuf>
	{
		let mut collector = Vec::new();

		Self::recursive_dir_inner(path, &mut collector);

		collector.into_iter()
	}

	fn recursive_dir_inner(path: &Path, collector: &mut Vec<PathBuf>)
	{
		fs::read_dir(path).unwrap().flatten().for_each(|entry|
		{
			let path = entry.path();
			if path.is_dir()
			{
				Self::recursive_dir_inner(&path, collector);
			} else
			{
				collector.push(entry.path());
			}
		})
	}
}

pub struct Assets
{
	textures: HashMap<String, Arc<RwLock<Texture>>>,
	models: HashMap<String, Arc<RwLock<Model>>>,
    default_models: Vec<Arc<RwLock<Model>>>
}

impl Assets
{
    pub fn new<TexturesPath, ModelsPath>(
        resource_uploader: &mut ResourceUploader,
        textures_path: Option<TexturesPath>,
        models_path: Option<ModelsPath>
    ) -> Self
    where
        TexturesPath: AsRef<Path>,
        ModelsPath: AsRef<Path>
    {
        let textures = Self::load_resource(textures_path, |path|
        {
            FilesLoader::load_images(path).map(|named_value|
            {
                named_value.map(|image| Texture::new(resource_uploader, image))
            })
        });

        let models = Self::load_resource(models_path, |path|
        {
            FilesLoader::load(path).map(|named_value|
            {
                named_value.map(|path| Model::load(path).unwrap())
            })
        });

        let default_models = Self::create_default_models();

        Self{textures, models, default_models}
    }

    fn load_resource<T, F, I, P>(maybe_path: Option<P>, f: F) -> HashMap<String, Arc<RwLock<T>>>
    where
        P: AsRef<Path>,
        I: Iterator<Item=NamedValue<T>>,
        F: FnOnce(P) -> I
    {
        maybe_path.map(|path|
        {
            f(path).map(|NamedValue{name, value}|
            {
                (name, Arc::new(RwLock::new(value)))
            }).collect()
        }).unwrap_or_default()
    }

    pub fn default_model(&self, id: DefaultModel) -> Arc<RwLock<Model>>
    {
        self.default_models[id as usize].clone()
    }

    pub fn texture(&self, name: &str) -> Arc<RwLock<Texture>>
    {
        self.textures.get(name).unwrap_or_else(||
        {
            panic!("no texture named '{}' found", name)
        }).clone()
    }

    pub fn model(&self, name: &str) -> Arc<RwLock<Model>>
    {
        self.models.get(name).unwrap_or_else(||
        {
            panic!("no model named '{}' found", name)
        }).clone()
    }

    pub fn add_textures<T>(&mut self, textures: T)
    where
        T: IntoIterator<Item=(String, Texture)>
    {
        Self::add_assets(textures.into_iter(), &mut self.textures);
    }

    pub fn add_models<T>(&mut self, models: T)
    where
        T: IntoIterator<Item=(String, Model)>
    {
        Self::add_assets(models.into_iter(), &mut self.models);
    }

    fn create_default_models() -> Vec<Arc<RwLock<Model>>>
    {
        DefaultModel::iter().map(|default_model|
        {
            let model = match default_model
            {
                DefaultModel::Square =>
                {
                    Model::square(1.0)
                }
            };

            Arc::new(RwLock::new(model))
        }).collect()
    }

    fn add_assets<AddAssetsType, T>(
        add_assets: AddAssetsType,
        assets: &mut HashMap<String, Arc<RwLock<T>>>
    )
    where
        AddAssetsType: Iterator<Item=(String, T)>
    {
        let insert_assets = add_assets.map(|(name, asset)|
        {
            (name, Arc::new(RwLock::new(asset)))
        }).collect::<Vec<_>>();

        assets.extend(insert_assets);
    }

	pub fn swap_pipeline(&mut self, info: &PipelineInfo)
	{
		self.textures.iter_mut().for_each(|(_name, texture)|
		{
			texture.write().swap_pipeline(info)
		});
	}
}

impl fmt::Debug for Assets
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        let texture_names = self.textures.keys().map(|x| x.to_owned())
            .reduce(|acc, v|
            {
                acc + ", " + &v
            }).unwrap_or_default();

        let model_names = self.models.keys().map(|x| x.to_owned())
            .reduce(|acc, v|
            {
                acc + ", " + &v
            }).unwrap_or_default();

        write!(f, "Assets {{ textures: [{}], models: [{}] }}", texture_names, model_names)
    }
}
