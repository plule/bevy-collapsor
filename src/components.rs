use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Default)]
pub struct ModelAssets {
    pub models: Vec<Handle<Scene>>,
    pub up_cube_mesh: Handle<Mesh>,
    pub up_cube_mat: Handle<StandardMaterial>,
    pub undecided_mesh: Handle<Mesh>,
    pub undecided_mat: Handle<StandardMaterial>,
    pub impossible_mesh: Handle<Mesh>,
    pub impossible_mat: Handle<StandardMaterial>,
}

#[derive(Inspectable, Clone, Copy, PartialEq, FromPrimitive, Hash, Eq, Debug)]
pub enum Orientation {
    NORTH = 0,
    EST,
    SOUTH,
    WEST,
}

impl Default for Orientation {
    fn default() -> Self {
        Orientation::NORTH
    }
}

impl From<Orientation> for Coordinates {
    fn from(orientation: Orientation) -> Self {
        match orientation {
            Orientation::NORTH => Coordinates::new(0, 1),
            Orientation::EST => Coordinates::new(1, 0),
            Orientation::SOUTH => Coordinates::new(0, -1),
            Orientation::WEST => Coordinates::new(-1, 0),
        }
    }
}

impl From<Orientation> for Quat {
    fn from(orientation: Orientation) -> Self {
        let angle = match orientation {
            Orientation::NORTH => 0.,
            Orientation::EST => -90.0_f32.to_radians(),
            Orientation::SOUTH => -180.0_f32.to_radians(),
            Orientation::WEST => -270.0_f32.to_radians(),
        };
        Quat::from_rotation_y(angle)
    }
}

impl Orientation {
    pub fn rotate(&mut self, amount: i32) {
        *self = FromPrimitive::from_i32(((*self as i32) + amount).rem_euclid(4)).unwrap();
    }
}

#[derive(Default, Component, Inspectable, Clone, Copy, PartialEq, Hash, Eq, Debug)]
pub struct TilePrototype {
    pub model_index: usize,
    pub orientation: Orientation,
}

#[derive(Default, Component, Inspectable, Clone, PartialEq)]
pub struct OptionalTilePrototype {
    pub tile_prototype: Option<TilePrototype>,
}

#[derive(Default, Component, Clone, PartialEq, Eq, Debug)]
pub struct MultiTilePrototype {
    pub tiles: HashSet<TilePrototype>,
}

#[derive(Default, Component, Inspectable, Clone, PartialEq)]
pub struct DrawTile {
    pub tile: OptionalTilePrototype,
}

impl OptionalTilePrototype {
    pub fn from_index(index: usize) -> OptionalTilePrototype {
        OptionalTilePrototype {
            tile_prototype: Some(TilePrototype {
                model_index: index,
                ..Default::default()
            }),
        }
    }
}

#[derive(Default, Inspectable)]
pub struct SelectedTileProto {
    pub tile_prototype: OptionalTilePrototype,
}

#[derive(Component, Inspectable, Default)]
pub struct RuleTileTag;

#[derive(Component, Inspectable)]
pub struct Palette {
    pub index: usize,
}

impl Palette {
    pub fn new(index: usize) -> Self {
        Self { index }
    }
}

#[derive(Component, Inspectable, Default)]
pub struct Coordinates {
    pub x: i32,
    pub y: i32,
}

impl Coordinates {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

pub struct RulesNeedUpdateEvent {}

#[derive(Default, Debug, Clone)]
pub struct Constraints {
    pub constraints: HashMap<Orientation, HashSet<TilePrototype>>,
}

#[derive(Component, Default, Clone)]
pub struct Connectivity {
    pub connectivity: HashMap<Orientation, Entity>,
}

pub struct Map {
    pub tile_models: Vec<String>,
    pub width: usize,
    pub height: usize,
}

#[derive(Default, Debug)]
pub struct Rules {
    pub constraints: HashMap<TilePrototype, Constraints>,
}

impl Map {
    pub fn new(width: usize, height: usize) -> Self {
        let tile_models = vec![
            "models/ground_grass.glb#Scene0".to_string(),
            "models/ground_pathBend.glb#Scene0".to_string(),
            "models/ground_pathCross.glb#Scene0".to_string(),
            "models/ground_pathEndClosed.glb#Scene0".to_string(),
            "models/ground_pathSplit.glb#Scene0".to_string(),
            "models/ground_pathStraight.glb#Scene0".to_string(),
        ];
        let mut tile_prototypes = HashSet::new();

        for orientation in [
            Orientation::NORTH,
            Orientation::EST,
            Orientation::SOUTH,
            Orientation::WEST,
        ] {
            for model_index in 0..tile_models.len() {
                tile_prototypes.insert(TilePrototype {
                    model_index,
                    orientation,
                });
            }
        }

        Self {
            tile_models,
            width,
            height,
        }
    }
}

#[cfg(test)]
#[test]
fn rotate_orientation() {
    let mut orientation = Orientation::NORTH;
    orientation.rotate(-2);
    assert!(orientation == Orientation::SOUTH);
    orientation.rotate(1);
    assert!(orientation == Orientation::WEST);
}
