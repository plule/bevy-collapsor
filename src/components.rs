use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

pub struct ModelAssets {
    pub up_cube_mesh: Handle<Mesh>,
    pub up_cube_mat: Handle<StandardMaterial>,
    pub undecided_mesh: Handle<Mesh>,
    pub undecided_mat: Handle<StandardMaterial>,
    pub impossible_mesh: Handle<Mesh>,
    pub impossible_mat: Handle<StandardMaterial>,
    pub pick_mesh: Handle<Mesh>,
    pub pick_mat: Handle<StandardMaterial>,
}

impl FromWorld for ModelAssets {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();
        let up_cube_mesh = meshes.add(shape::Cube { size: 0.1 }.into());
        let undecided_mesh = meshes.add(shape::Plane { size: 1.0 }.into());
        let impossible_mesh = meshes.add(shape::Plane { size: 1.0 }.into());
        let pick_mesh = meshes.add(Mesh::from(shape::Plane { size: 1.0 }));

        let mut materials = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();
        let up_cube_mat = materials.add(Color::RED.into());
        let undecided_mat = materials.add(Color::BLACK.into());
        let impossible_mat = materials.add(Color::RED.into());
        let pick_mat = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            ..Default::default()
        });

        Self {
            up_cube_mesh,
            up_cube_mat,
            undecided_mesh,
            undecided_mat,
            impossible_mesh,
            impossible_mat,
            pick_mesh,
            pick_mat,
        }
    }
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

#[derive(Default, Component, Clone, PartialEq, Hash, Eq, Debug)]
pub struct TilePrototype {
    pub index: usize,
    pub model: Handle<Scene>,
    pub equivalences: Equivalences,
}

impl TilePrototype {
    pub fn new(index: usize, model: Handle<Scene>, equivalences: Equivalences) -> Self {
        Self {
            index,
            model,
            equivalences,
        }
    }

    pub fn equivalent_directions(&self, direction: Orientation) -> Vec<Orientation> {
        let equivalent_rotations = match self.equivalences {
            Equivalences::None => vec![0],
            Equivalences::HalfTurn => vec![0, 2],
            Equivalences::QuarterTurn => vec![0, 1, 2, 3, 4],
        };

        equivalent_rotations
            .iter()
            .map(|rotation| direction.rotated(*rotation))
            .collect()
    }

    pub fn available_rotations(&self) -> Vec<i32> {
        match self.equivalences {
            Equivalences::None => vec![0, 1, 2, 3],
            Equivalences::HalfTurn => vec![0, 1],
            Equivalences::QuarterTurn => vec![0],
        }
    }

    pub fn make_tile(&self, orientation: Orientation) -> Tile {
        Tile::new(self.index, orientation)
    }

    pub fn make_rotated_tile(&self, original_orientation: Orientation, rotation: i32) -> Tile {
        let orientation = original_orientation.rotated(rotation);
        let orientation = match self.equivalences {
            Equivalences::None => orientation,
            Equivalences::HalfTurn => match orientation {
                Orientation::North | Orientation::South => Orientation::North,
                Orientation::East | Orientation::West => Orientation::East,
            },
            Equivalences::QuarterTurn => Orientation::North,
        };
        self.make_tile(orientation)
    }
}

#[derive(Default, Component, Inspectable, Clone, Copy, PartialEq, Hash, Eq, Debug)]
pub struct Tile {
    pub prototype_index: usize,
    pub orientation: Orientation,
}

impl Tile {
    pub fn new(prototype_index: usize, orientation: Orientation) -> Self {
        Self {
            prototype_index,
            orientation,
        }
    }
}

#[derive(Default, Component, Inspectable, Clone, PartialEq)]
pub struct OptionalTile {
    pub tile: Option<Tile>,
}

impl OptionalTile {
    pub fn new(tile: Option<Tile>) -> Self {
        Self { tile }
    }
}

/// Superposition of possible states
///
/// If the tiles size is 1, then it's resolved.
/// If it's zero, then it's impossible.
#[derive(Default, Component, Clone, PartialEq, Eq, Debug)]
pub struct TileSuperposition {
    pub tiles: HashSet<Tile>,
}

#[derive(Default, Component, Inspectable, Clone, PartialEq)]
pub struct DrawTile {
    pub tile: OptionalTile,
}

impl From<Tile> for OptionalTile {
    fn from(tile: Tile) -> Self {
        Self { tile: Some(tile) }
    }
}

#[derive(Default)]
pub struct TileSelection {
    pub rotation: i32,
    pub prototype: Option<TilePrototype>,
}

impl TileSelection {
    pub fn make_tile(&self) -> Option<Tile> {
        match &self.prototype {
            Some(prototype) => Some(
                prototype
                    .clone()
                    .make_rotated_tile(Orientation::North, self.rotation),
            ),
            None => None,
        }
    }
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
    pub constraints: HashMap<Orientation, HashSet<Tile>>,
}

#[derive(Component, Default, Clone)]
pub struct Connectivity {
    pub connectivity: HashMap<Orientation, Entity>,
}

#[derive(Debug)]
pub struct Rules {
    pub width: usize,
    pub height: usize,
    pub prototypes: Vec<TilePrototype>,
    pub constraints: HashMap<Tile, Constraints>,
}

impl FromWorld for Rules {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
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

        let mut prototypes = Vec::new();
        for index in 0..palette.len() {
            let elt = &palette[index];
            let model = asset_server.load(&elt.tile_model);
            prototypes.push(TilePrototype::new(index, model, elt.equivalences))
        }

        Self {
            width: 32,
            height: 32,
            prototypes,
            constraints: Default::default(),
        }
    }
}

struct PaletteElement {
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

#[cfg(test)]
#[test]
fn rotate_orientation() {
    let mut orientation = Orientation::North;
    orientation.rotate(-2);
    assert!(orientation == Orientation::South);
    orientation.rotate(1);
    assert!(orientation == Orientation::West);
}
