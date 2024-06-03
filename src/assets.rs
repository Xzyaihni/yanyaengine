use std::{
    fs,
    fmt,
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    ops::{Index, IndexMut}
};

use parking_lot::RwLock;

use strum::IntoEnumIterator;
use strum_macros::{EnumIter, IntoStaticStr};

use serde::{Serialize, Deserialize};

use crate::{
    PipelineInfo,
    UniformLocation,
    object::{
        resource_uploader::ResourceUploader,
        model::Model,
        texture::{RgbaImage, Texture}
    }
};


#[derive(EnumIter, IntoStaticStr)]
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

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct TextureId(usize);

impl From<usize> for TextureId
{
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

impl From<TextureId> for usize
{
    fn from(value: TextureId) -> usize
    {
        value.0
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelId(usize);

impl From<usize> for ModelId
{
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

impl From<ModelId> for usize
{
    fn from(value: ModelId) -> usize
    {
        value.0
    }
}

struct IdsStorage<I, T>
{
    ids: HashMap<String, I>,
    data: Vec<T>
}

impl<I, T> FromIterator<(String, T)> for IdsStorage<I, T>
where
    I: From<usize> + Clone
{
    fn from_iter<Iter>(iter: Iter) -> Self
    where
        Iter: IntoIterator<Item=(String, T)>
    {
        let mut this = Self::default();

        this.extend(iter);

        this
    }
}

impl<I, T> Extend<(String, T)> for IdsStorage<I, T>
where
    I: From<usize> + Clone
{
    fn extend<Iter>(&mut self, iter: Iter)
    where
        Iter: IntoIterator<Item=(String, T)>
    {
        iter.into_iter().for_each(|item| { self.insert(item); });
    }
}

impl<I, T> Default for IdsStorage<I, T>
{
    fn default() -> Self
    {
        Self{ids: HashMap::new(), data: Vec::new()}
    }
}

impl<I, T> IdsStorage<I, T>
{
    pub fn insert(&mut self, item: (String, T)) -> I
    where
        I: From<usize> + Clone
    {
        let id: I = self.data.len().into();

        self.ids.insert(item.0, id.clone());
        self.data.push(item.1);

        id
    }

    pub fn get_id(&self, name: &str) -> &I
    {
        self.ids.get(name).unwrap_or_else(|| panic!("asset named `{name}` doesnt exist"))
    }

    pub fn keys(&self) -> impl Iterator<Item=&String>
    {
        self.ids.keys()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut T>
    {
        self.data.iter_mut()
    }
}

impl<I, T> Index<I> for IdsStorage<I, T>
where
    I: Into<usize>
{
    type Output = T;

    fn index(&self, index: I) -> &Self::Output
    {
        &self.data[index.into()]
    }
}

impl<I, T> IndexMut<I> for IdsStorage<I, T>
where
    I: Into<usize>
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output
    {
        &mut self.data[index.into()]
    }
}

pub struct Assets
{
    textures: IdsStorage<TextureId, Arc<RwLock<Texture>>>,
	models: IdsStorage<ModelId, Arc<RwLock<Model>>>
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
        let texture_location = UniformLocation{set: 0, binding: 0};
        let textures = Self::load_resource(textures_path, |path|
        {
            FilesLoader::load_images(path).map(|named_value|
            {
                named_value.map(|image| Texture::new(resource_uploader, image, texture_location))
            })
        });

        let mut models = Self::load_resource(models_path, |path|
        {
            FilesLoader::load(path).map(|named_value|
            {
                named_value.map(|path| Model::load(path).unwrap())
            })
        });

        models.extend(Self::create_default_models());

        Self{textures, models}
    }

    fn load_resource<Id, T, F, I, P>(maybe_path: Option<P>, f: F) -> IdsStorage<Id, Arc<RwLock<T>>>
    where
        Id: From<usize> + Clone,
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

    pub fn default_model(&self, id: DefaultModel) -> ModelId
    {
        self.model_id(id.into())
    }

    pub fn texture_id(&self, name: &str) -> TextureId
    {
        *self.textures.get_id(name)
    }

    pub fn texture_by_name(&self, name: &str) -> &Arc<RwLock<Texture>>
    {
        &self.textures[*self.textures.get_id(name)]
    }

    pub fn texture(&self, id: TextureId) -> &Arc<RwLock<Texture>>
    {
        &self.textures[id]
    }

    pub fn model_id(&self, name: &str) -> ModelId
    {
        *self.models.get_id(name)
    }

    pub fn model_by_name(&self, name: &str) -> &Arc<RwLock<Model>>
    {
        &self.models[*self.models.get_id(name)]
    }

    pub fn model(&self, id: ModelId) -> &Arc<RwLock<Model>>
    {
        &self.models[id]
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

    fn create_default_models() -> impl Iterator<Item=(String, Arc<RwLock<Model>>)>
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

            let name: &str = default_model.into();
            (name.to_owned(), Arc::new(RwLock::new(model)))
        })
    }

    fn add_assets<AddAssetsType, I, T>(
        add_assets: AddAssetsType,
        assets: &mut IdsStorage<I, Arc<RwLock<T>>>
    )
    where
        I: From<usize> + Clone,
        AddAssetsType: Iterator<Item=(String, T)>
    {
        let insert_assets = add_assets.map(|(name, asset)|
        {
            (name, Arc::new(RwLock::new(asset)))
        });

        assets.extend(insert_assets);
    }

	pub fn swap_pipeline(&mut self, info: &PipelineInfo)
	{
		self.textures.iter_mut().for_each(|texture|
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
