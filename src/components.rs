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

/// Variation of the palette elements that are equivalents
#[derive(Component, Inspectable, Clone, Copy, PartialEq, Hash, Eq, Debug)]
pub enum Equivalences {
    None,
    HalfTurn,
    QuarterTurn,
}

impl Default for Equivalences {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Inspectable, Clone, Copy, PartialEq, FromPrimitive, Hash, Eq, Debug)]
pub enum Orientation {
    North = 0,
    East,
    South,
    West,
}

impl Default for Orientation {
    fn default() -> Self {
        Orientation::North
    }
}

impl From<Orientation> for Quat {
    fn from(orientation: Orientation) -> Self {
        let angle = match orientation {
            Orientation::North => 0.,
            Orientation::East => -90.0_f32.to_radians(),
            Orientation::South => -180.0_f32.to_radians(),
            Orientation::West => -270.0_f32.to_radians(),
        };
        Quat::from_rotation_y(angle)
    }
}

impl Orientation {
    pub fn values() -> [Orientation; 4] {
        [
            Orientation::North,
            Orientation::East,
            Orientation::South,
            Orientation::West,
        ]
    }

    pub fn rotate(&mut self, amount: i32) {
        *self = FromPrimitive::from_i32(((*self as i32) + amount).rem_euclid(4)).unwrap();
    }

    pub fn rotated(&self, amount: i32) -> Self {
        let mut ret = self.clone();
        ret.rotate(amount);
        ret
    }

    pub fn offset(&self, coordinate: &Coordinates) -> Coordinates {
        match self {
            Orientation::North => Coordinates::new(coordinate.x, coordinate.y + 1),
            Orientation::East => Coordinates::new(coordinate.x - 1, coordinate.y),
            Orientation::South => Coordinates::new(coordinate.x, coordinate.y - 1),
            Orientation::West => Coordinates::new(coordinate.x + 1, coordinate.y),
        }
    }
}

#[derive(Default, Component, Inspectable, Clone, Copy, PartialEq, Hash, Eq, Debug)]
pub struct TilePrototype {
    pub model_index: usize,
    pub orientation: Orientation,
    pub equivalences: Equivalences,
}

impl TilePrototype {
    pub fn new(model_index: usize, orientation: Orientation, equivalences: Equivalences) -> Self {
        Self {
            model_index,
            orientation,
            equivalences,
        }
    }

    pub fn rotated(&self, amount: i32) -> Self {
        TilePrototype {
            model_index: self.model_index,
            orientation: self.orientation.rotated(amount),
            equivalences: self.equivalences,
        }
    }

    /// Get all the tile prototypes equivalent.
    pub fn equivalences(&self) -> Vec<Self> {
        let rotations = match self.equivalences {
            Equivalences::None => vec![0],
            Equivalences::HalfTurn => vec![0, 2],
            Equivalences::QuarterTurn => vec![0, 1, 2, 3],
        };

        rotations
            .iter()
            .map(|rotation| self.rotated(*rotation))
            .collect()
    }
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

impl From<TilePrototype> for OptionalTilePrototype {
    fn from(prototype: TilePrototype) -> Self {
        Self {
            tile_prototype: Some(prototype),
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
pub struct PaletteTag {}

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

#[derive(Default, Debug)]
pub struct Rules {
    pub constraints: HashMap<TilePrototype, Constraints>,
}

pub struct PaletteElement {
    pub tile_model: String,
    pub equivalences: Equivalences,
}

impl PaletteElement {
    pub fn new(tile_model: &str, symmetry: Equivalences) -> Self {
        Self {
            tile_model: tile_model.to_string(),
            equivalences: symmetry,
        }
    }
}

pub struct Map {
    pub palette: Vec<PaletteElement>,
    pub width: usize,
    pub height: usize,
}

impl Map {
    pub fn new(width: usize, height: usize) -> Self {
        let palette = vec![
            PaletteElement::new("models/bridge_wood.glb#Scene0", Equivalences::HalfTurn),
            PaletteElement::new("models/ground_grass.glb#Scene0", Equivalences::QuarterTurn),
            PaletteElement::new("models/ground_pathBend.glb#Scene0", Equivalences::None),
            PaletteElement::new(
                "models/ground_pathCross.glb#Scene0",
                Equivalences::QuarterTurn,
            ),
            PaletteElement::new("models/ground_pathEndClosed.glb#Scene0", Equivalences::None),
            PaletteElement::new("models/ground_pathSplit.glb#Scene0", Equivalences::None),
            PaletteElement::new(
                "models/ground_pathStraight.glb#Scene0",
                Equivalences::HalfTurn,
            ),
            PaletteElement::new("models/ground_riverBendBank.glb#Scene0", Equivalences::None),
            PaletteElement::new("models/ground_riverCorner.glb#Scene0", Equivalences::None),
            PaletteElement::new(
                "models/ground_riverCross.glb#Scene0",
                Equivalences::QuarterTurn,
            ),
            PaletteElement::new(
                "models/ground_riverCornerSmall.glb#Scene0",
                Equivalences::None,
            ),
            PaletteElement::new(
                "models/ground_riverEndClosed.glb#Scene0",
                Equivalences::None,
            ),
            PaletteElement::new(
                "models/ground_riverOpen.glb#Scene0",
                Equivalences::QuarterTurn,
            ),
            PaletteElement::new("models/ground_riverSide.glb#Scene0", Equivalences::None),
            PaletteElement::new("models/ground_riverSideOpen.glb#Scene0", Equivalences::None),
            PaletteElement::new("models/ground_riverSplit.glb#Scene0", Equivalences::None),
            PaletteElement::new(
                "models/ground_riverStraight.glb#Scene0",
                Equivalences::HalfTurn,
            ),
        ];

        Self {
            palette,
            width,
            height,
        }
    }
}

#[cfg(test)]
#[test]
fn rotate_orientation() {
    let mut orientation = Orientation::North;
    orientation.rotate(-2);
    assert!(orientation == Orientation::South);
    orientation.rotate(1);
    assert!(orientation == Orientation::West);
}
