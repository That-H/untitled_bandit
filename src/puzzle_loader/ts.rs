//! Contains a structure for converting characters into bandit compatible objects.

use crate::{
    entity::{En, EntityTemplate},
    templates::PLAYER_CHARACTER,
    *,
};
use std::collections::HashMap;

/// An object in a bandit map.
pub enum BanditObj {
    Tile(Tile),
    En(entity::En),
}

/// Mapping of characters to tiles or entities.
pub struct TileSet(pub HashMap<char, BanditObj>);

impl TileSet {
    /// Create an empty tile set.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Use the provided templates to create a mapping from each of their characters to the entity
    /// constructed. Assumes the template represented with the PLAYER_CHARACTER is the player
    /// template.
    pub fn add_temps(&mut self, templates: &[EntityTemplate]) {
        for temp in templates {
            let ch = *temp.ch.content();
            let is_player = ch == PLAYER_CHARACTER;
            let mut en = En::from_template(temp, is_player, false);
            if !is_player {
                en.acted = true;
            }
            self.0.insert(ch, BanditObj::En(en));
        }
    }

    /// Insert an object into the tile set.
    pub fn insert(&mut self, ch: char, obj: BanditObj) {
        self.0.insert(ch, obj);
    }

    /// Add the tile into the tile set.
    pub fn add_tile(&mut self, tile: Tile) {
        self.0.insert(*tile.repr().content(), BanditObj::Tile(tile));
    }

    /// Add the entity into the tile set.
    pub fn add_entity(&mut self, en: En) {
        let ch = *en.ch.content();
        self.0.insert(ch, BanditObj::En(en));
    }

    /// Get a reference to what would the given character should be, if there is one.
    pub fn map(&self, ch: char) -> Option<&BanditObj> {
        self.0.get(&ch)
    }
}
