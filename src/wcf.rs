use std::collections::{HashMap, HashSet};

use crate::components::*;
use bevy::prelude::*;
use rand::prelude::SliceRandom;
use std::hash::Hash;

pub struct WCFPlugin;

impl Plugin for WCFPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(observe_system)
            .add_system(collapse)
            .add_system(update_rules);
    }
}

/// Safe tile get from indexes
fn get_tile_prototype(map: &Vec<Vec<OptionalTile>>, coordinates: &Coordinates) -> Option<Tile> {
    if coordinates.x < 0 || coordinates.y < 0 {
        return None;
    }
    let line = map.get(coordinates.x as usize)?;
    let tile = line.get(coordinates.y as usize)?;
    tile.tile.clone()
}

fn expand_with_rotations(
    constraints: &HashMap<Tile, Allowed>,
    prototypes: &Vec<Prototype>,
) -> HashMap<Tile, Allowed> {
    let mut expanded = HashMap::<Tile, Allowed>::new();

    for (tile, tile_constraints) in constraints.iter() {
        let prototype = &prototypes[tile.prototype_index];
        for tile_rotations in 0..Orientation::values().len() as i32 {
            let rotated_tile = prototype.make_rotated_tile(tile.orientation, tile_rotations);
            let new_variant_constraints_entry = expanded.entry(rotated_tile).or_default();

            for (orientation, allowed_values) in tile_constraints.allowed.iter() {
                let new_constraints_entry: &mut HashSet<Tile> = new_variant_constraints_entry
                    .allowed
                    .entry(orientation.rotated(tile_rotations))
                    .or_default();
                for allowed_tile in allowed_values.iter() {
                    let prototype = &prototypes[allowed_tile.prototype_index];
                    let rotated_allowed_tile =
                        prototype.make_rotated_tile(allowed_tile.orientation, tile_rotations);
                    new_constraints_entry.insert(rotated_allowed_tile);
                }
            }
        }
    }

    expanded
}

fn intersection<T: Eq + Hash>(a: HashSet<T>, b: &HashSet<T>) -> HashSet<T> {
    a.into_iter().filter(|e| b.contains(e)).collect()
}

fn update_rules(
    mut rules: ResMut<Rules>,
    rules_query: Query<(&OptionalTile, &Coordinates), With<RuleTileTag>>,
    mut event_reader: EventReader<RulesNeedUpdateEvent>,
    mut tiles_query: Query<(Entity, &mut TileSuperposition, &Connectivity)>,
) {
    if !event_reader.is_empty() {
        info!("Rules changed, clearing");
        for _ in event_reader.iter() {}
        // Rule change

        // Read the rule map
        let rule_width = 16;
        let rule_height = 16;
        let mut rule_tiles = vec![vec![OptionalTile::default(); rule_width]; rule_height];
        for (tile, coordinates) in rules_query.iter() {
            rule_tiles[coordinates.x as usize][coordinates.y as usize] = tile.clone();
        }

        // Store the rule connectivities as constraints
        rules.alloweds = HashMap::<Tile, Allowed>::new();
        for x in 0..rule_width {
            for y in 0..rule_height {
                let tile = &rule_tiles[x][y];
                let coords = Coordinates::new(x as i32, y as i32);
                if let Some(tile) = &tile.tile {
                    let allowed = &mut rules.alloweds.entry(tile.clone()).or_default().allowed;

                    for orientation in Orientation::values() {
                        let neighbour_coords = orientation.offset(&coords);
                        let neighbour_tile = get_tile_prototype(&rule_tiles, &neighbour_coords);
                        if let Some(neighbour_tile) = neighbour_tile {
                            allowed
                                .entry(orientation)
                                .or_default()
                                .insert(neighbour_tile);
                        }
                    }
                }
            }
        }
        rules.alloweds = expand_with_rotations(&rules.alloweds, &rules.prototypes);

        // Reset to every possibilities on rule change
        let mut possible_tiles = HashSet::new();
        for tile in rules.alloweds.keys() {
            possible_tiles.insert(tile.clone());
        }
        for (_, mut multi_tile_prototype, _) in tiles_query.iter_mut() {
            multi_tile_prototype.tiles = possible_tiles.clone();
        }
    }
}

fn observe_system(mut query: Query<(Entity, &mut TileSuperposition)>) {
    let mut rng = rand::thread_rng();
    // Find the smallest > 1 entropy
    let mut min_entropy_entities = Vec::new();
    let mut min_entropy = usize::MAX;

    for (entity, wave) in query.iter() {
        if wave.dirty {
            // ongoing propagation
            return;
        }
        let entropy = wave.tiles.len();
        if entropy < min_entropy && entropy > 1 {
            min_entropy = entropy;
            min_entropy_entities.clear();
        }

        if entropy == min_entropy {
            min_entropy_entities.push(entity);
        }
    }

    let min_entropy_entity = match min_entropy_entities.choose(&mut rng) {
        Some(e) => *e,
        // Solved or impossible
        None => return,
    };

    let mut min_entropy_wave = query
        .get_component_mut::<TileSuperposition>(min_entropy_entity)
        .unwrap();

    // Observe the tile with the smallest entropy
    let min_entropy_tiles: Vec<&Tile> = min_entropy_wave.tiles.iter().collect();
    let observed = *min_entropy_tiles.choose(&mut rng).unwrap().clone();
    min_entropy_wave.tiles.clear();
    min_entropy_wave.tiles.insert(observed.clone());

    // Signal for propagation
    min_entropy_wave.dirty = true;
}

fn collapse(
    rules: Res<Rules>,
    tuning: Res<Tuning>,
    mut query: Query<(Entity, &mut TileSuperposition, &Connectivity)>,
) {
    for _ in 0..tuning.collapse_per_frame {
        // Find a dirty wave
        let mut propagating_entity = Option::<Entity>::default();
        for (entity, wave, _) in query.iter() {
            if wave.dirty {
                propagating_entity = Some(entity);
                break;
            }
        }

        let propagating_entity = match propagating_entity {
            Some(e) => e,
            None => return,
        };

        // Get all its allowed values and its connectivity
        let (_, propagating_wave, propagating_connectivity) =
            query.get(propagating_entity).unwrap();

        let propagating_wave = propagating_wave.tiles.clone();
        let propagating_connectivity = propagating_connectivity.connectivity.clone();

        // Find its neighbours
        for orientation in Orientation::values() {
            if let Some(neighbour) = propagating_connectivity.get(&orientation) {
                let neighbour_wave = &mut query
                    .get_component_mut::<TileSuperposition>(*neighbour)
                    .unwrap();

                // Skip if the neighbour is already resolved or impossible
                if neighbour_wave.tiles.len() <= 1 {
                    continue;
                }

                // Sum all the possible values for this neighbour given its own allowed values
                let mut all_allowed_neighbour = HashSet::<Tile>::new();
                for value in &propagating_wave {
                    let rule_constraints =
                        rules.alloweds.get(value).unwrap().allowed.get(&orientation);
                    if let Some(allowed_neighbour) = rule_constraints {
                        all_allowed_neighbour.extend(allowed_neighbour);
                    }
                }

                // Intersect the previous list of allowed values with the new constraints
                let new_allowed_values =
                    intersection(all_allowed_neighbour, &mut neighbour_wave.tiles);

                // If impacted, update the tile and mark it as dirty for propagation
                if &new_allowed_values != &neighbour_wave.tiles {
                    neighbour_wave.tiles = new_allowed_values;
                    if !neighbour_wave.tiles.is_empty() {
                        // Don't propagate impossibility
                        neighbour_wave.dirty = true;
                    }
                }
            }
        }

        // Mark the entity as not dirty
        let mut wave = query
            .get_component_mut::<TileSuperposition>(propagating_entity)
            .unwrap();
        wave.dirty = false;
    }
}
