use std::collections::{HashMap, HashSet};

use crate::components::*;
use bevy::prelude::*;
use rand::prelude::SliceRandom;
use std::hash::Hash;

pub struct WCFPlugin;

impl Plugin for WCFPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(collapse);
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

fn collapse(
    mut rules: ResMut<Rules>,
    rules_query: Query<(&OptionalTile, &Coordinates), With<RuleTileTag>>,
    mut event_reader: EventReader<RulesNeedUpdateEvent>,
    mut tiles_query: Query<(Entity, &mut TileSuperposition, &Connectivity)>,
) {
    let mut rng = rand::thread_rng();

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

    // Store locally the state
    let mut entity_indexes = HashMap::<Entity, usize>::new();
    let mut entities = Vec::new();
    let mut index: usize = 0;
    for (entity, _, _) in tiles_query.iter() {
        entity_indexes.insert(entity, index);
        entities.push(entity);
        index += 1;
    }
    let count = index;

    let mut waves = Vec::new();
    let mut connectivities = Vec::new();
    for (_, multi_line_prototype, connectivity) in tiles_query.iter() {
        waves.push(multi_line_prototype.tiles.clone());
        let mut connectivity_by_index = HashMap::new();
        for (orientation, entity) in connectivity.connectivity.iter() {
            connectivity_by_index.insert(*orientation, *entity_indexes.get(entity).unwrap());
        }
        connectivities.push(connectivity_by_index);
    }

    // Find the smallest > 1 entropy
    let mut min_entropy_entities = Vec::new();
    let mut min_entropy = usize::MAX;

    for i in 0..count {
        let entropy = waves[i].len();
        if entropy < min_entropy && entropy > 1 {
            min_entropy = entropy;
            min_entropy_entities.clear();
        }

        if entropy == min_entropy {
            min_entropy_entities.push(i);
        }
    }
    let min_entropy_entity = min_entropy_entities.choose(&mut rng);

    if let Some(min_entropy_entity) = min_entropy_entity {
        let min_entropy_entity = *min_entropy_entity;
        // Observe the tile with the smallest entropy
        observe(&mut waves[min_entropy_entity], &mut rng);

        // Propagate
        let mut need_propagation = HashSet::<usize>::new();
        need_propagation.insert(min_entropy_entity);
        while !need_propagation.is_empty() {
            // Pop an entity needing propagation
            let propagating_entity = need_propagation.iter().next().cloned().unwrap();
            need_propagation.take(&propagating_entity).unwrap();

            // Get all its allowed values and its connectivity
            let propagating_wave = waves[propagating_entity].clone();

            if propagating_wave.is_empty() {
                // Impossible to solve
                // Avoid propagating it everywhere
                continue;
            }

            let propagating_connectivity = connectivities[propagating_entity].clone();

            // Find its neighbours
            for orientation in Orientation::values() {
                if let Some(neighbour) = propagating_connectivity.get(&orientation) {
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
                        intersection(all_allowed_neighbour, &waves[*neighbour]);

                    // If impacted, update the tile and add it to the list needing propagation
                    if &new_allowed_values != &waves[*neighbour] {
                        need_propagation.insert(*neighbour);
                        waves[*neighbour].clear();
                        waves[*neighbour].extend(new_allowed_values.iter());
                    }
                }
            }
        }
    }

    // Apply the result to the entities
    for i in 0..count {
        let mut multitiles = tiles_query
            .get_component_mut::<TileSuperposition>(entities[i])
            .unwrap();
        if multitiles.tiles != waves[i] {
            multitiles.tiles = waves[i].clone();
        }
    }
}

fn observe(multi_tile_prototype: &mut HashSet<Tile>, rng: &mut rand::prelude::ThreadRng) {
    let tile_vec: Vec<&Tile> = multi_tile_prototype.iter().collect();
    let observed = *tile_vec.choose(rng).unwrap().clone();
    multi_tile_prototype.clear();
    multi_tile_prototype.insert(observed.clone());
}
