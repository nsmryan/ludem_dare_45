use quicksilver::prelude::*;

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
enum GameState {
    Playing,
    Lost,
}

#[derive(Clone, Debug, PartialEq)]
struct Tile {
    pos: Vector,
    glyph: char,
    color: Color,
    blocks: bool,
}

type Hp = i32;

type EntityId = usize;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Trap {
    Berserk,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Monster {
    hp: Hp,
    max_hp: Hp,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Player {
    hp: Hp,
    max_hp: Hp,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum EntityType {
    Trap(Trap),
    Monster(Monster),
    Player(Player),
}

impl EntityType {
    fn monster(max_hp: Hp) -> EntityType {
        return EntityType::Monster(Monster { hp: max_hp, max_hp: max_hp });
    }

    fn trap(trap: Trap) -> EntityType {
        return EntityType::Trap(trap);
    }

    fn is_monster(&self) -> bool {
        return match self {
            EntityType::Monster(_) => true,
            _ => false,
        };
    }

    fn is_player(&self) -> bool {
        return match self {
            EntityType::Player(_) => true,
            _ => false,
        };
    }

    fn is_trap(&self) -> bool {
        return match self {
            EntityType::Trap(_) => true,
            _ => false,
        };
    }

    fn lose_hp(&mut self, amount: Hp) {
        match self {
            EntityType::Player(player) => {
                player.hp -= amount;
            },

            EntityType::Monster(monster) => {
                monster.hp -= amount;
            },

            _ => panic!("This entity cannot lose HP!"),
        }
    }
}

trait HasHp {
    fn max_hp(&self) -> Hp;
    fn hp(&self) -> Hp;
}

impl HasHp for Player {
    fn max_hp(&self) -> Hp {
        return self.max_hp;
    }

    fn hp(&self) -> Hp {
        return self.max_hp;
    }
}

impl HasHp for Monster {
    fn max_hp(&self) -> Hp {
        return self.max_hp;
    }

    fn hp(&self) -> Hp {
        return self.max_hp;
    }
}

impl HasHp for Entity {
    fn max_hp(&self) -> Hp {
        return match &self.typ {
            EntityType::Monster(monster) => monster.max_hp,
            EntityType::Player(player) => player.max_hp,
            _ => panic!("Tried to get hp from entity with no HP!"),
        };
    }

    fn hp(&self) -> Hp {
        return match &self.typ {
            EntityType::Monster(monster) => monster.hp,
            EntityType::Player(player) => player.hp,
            _ => panic!("Tried to get hp from entity with no HP!"),
        };
    }
}

type Map = Vec<Tile>;

const TEXT_COLOR: Color = Color::WHITE;
const BACKGROUND_COLOR: Color = Color::BLACK;

const MAP_WIDTH: usize = 20;
const MAP_HEIGHT: usize = 15;

const MAP_DRAW_X_OFFSET: usize  = 50;
const MAP_DRAW_Y_OFFSET: usize  = 120;

fn generate_map(size: Vector) -> Vec<Tile> {
    let width = size.x as usize;
    let height = size.y as usize;
    let mut map = Vec::with_capacity(width * height);
    for x in 0..width {
        for y in 0..height {
            let mut tile = Tile {
                pos: Vector::new(x as f32, y as f32),
                glyph: ' ',
                color: TEXT_COLOR,
                blocks: false,
            };

            if x == 0 || x == width - 1 || y == 0 || y == height - 1 {
                tile.glyph = '#';
                tile.blocks = true;
            };
            map.push(tile);
        }
    }
    map
}

fn blocked_tile(pos: Vector, map: &Map) -> bool {
    return map.iter().any(|tile| tile.blocks && tile.pos == pos);
}

fn occupied_tile(pos: Vector, entities: &Vec<Entity>) -> Option<Entity> {
    return entities.iter().find(|entity| entity.pos == pos).map(|entity| entity.clone());
}

fn clamp(min: f32, max: f32, value: f32) -> f32 {
    let result: f32;

    if value < min {
        result = min;
    } else if value > max {
        result = max;
    } else {
        result = value;
    }

    return result;
}

#[derive(Clone, Debug, PartialEq)]
struct Entity {
    pos: Vector,
    glyph: char,
    color: Color,
    typ: EntityType,
}

impl Entity {
    fn goblin(pos: Vector) -> Entity {
        Entity {
            pos: pos,
            glyph: 'g',
            color: Color::RED,
            typ: EntityType::monster(1),
        }
    }
}

fn generate_entities(entities: &mut Vec<Entity>) {
    entities.push(Entity::goblin(Vector::new(9, 6)));
    entities.push(Entity::goblin(Vector::new(2, 4)));
}

struct Game {
    game_state: GameState,
    title: Asset<Image>,
    mononoki_font_info: Asset<Image>,
    square_font_info: Asset<Image>,
    lost_game_message: Asset<Image>,
    char_map: HashMap<u32, Image>,
    inventory: Asset<Image>,
    map_size: Vector,
    map: Map,
    entities: Vec<Entity>,
    player_id: usize,
    tileset: Asset<HashMap<char, Image>>,
    tile_size_px: Vector,
}

impl State for Game {
    /// Load the assets and initialise the game
    fn new() -> Result<Self> {
        // The Mononoki font: https://madmalik.github.io/mononoki/
        // License: SIL Open Font License 1.1
        let font_mononoki = "mononoki-Regular.ttf";

        let title = Asset::new(Font::load(font_mononoki).and_then(|font| {
            font.render("Ludem Dare 45", &FontStyle::new(72.0, TEXT_COLOR))
        }));

        let font_image = "rexpaint16x16.png";
        let mut char_map = HashMap::new();
        let char_size = Vector::new(16, 16);
        Image::load(font_image).map(|image| {
            for char_ix in 0..256 {
                let char_x = char_ix % 16;
                let char_y = char_ix / 16;
                let char_pos = Vector::new(char_x * 16, char_y * 16);
                char_map.insert(char_ix, image.subimage(Rectangle::new(char_pos, char_size)));
            }
        }).map_err(|err| {
            panic!("Error loading font image: {}", err);
        }).poll().unwrap();

        let lost_game_message = Asset::new(Font::load(font_mononoki).and_then(|font| {
            font.render("You Lose!", &FontStyle::new(72.0, TEXT_COLOR))
        }));

        let mononoki_font_info = Asset::new(Font::load(font_mononoki).and_then(|font| {
            font.render(
                "",
                &FontStyle::new(20.0, TEXT_COLOR),
            )
        }));

        let square_font_info = Asset::new(Font::load(font_mononoki).and_then(move |font| {
            font.render(
                "A Ludem Dare Game by Joel and Noah Ryan",
                &FontStyle::new(20.0, TEXT_COLOR),
            )
        }));

        let inventory = Asset::new(Font::load(font_mononoki).and_then(move |font| {
            font.render(
                "Inventory:\n[A] Sword\n[B] Shield\n[C] Darts",
                &FontStyle::new(20.0, TEXT_COLOR),
            )
        }));

        let map_size = Vector::new(MAP_WIDTH as u8, MAP_HEIGHT as u8);
        let map = generate_map(map_size);
        let player_id = 0;

        let mut entities = Vec::new();
        entities.push(Entity {
            pos: Vector::new(5, 3),
            glyph: '@',
            color: Color::ORANGE,
            typ: EntityType::Player(Player { hp: 3, max_hp: 5 }),
        });
        generate_entities(&mut entities);

        // The Square font: http://strlen.com/square/?s[]=font
        // License: CC BY 3.0 https://creativecommons.org/licenses/by/3.0/deed.en_US
        let font_square = "square.ttf";
        let game_glyphs = "#@g.%";
        let tile_size_px = Vector::new(24, 24);
        let tileset = Asset::new(Font::load(font_square).and_then(move |text| {
            let tiles = text
                .render(game_glyphs, &FontStyle::new(tile_size_px.y, TEXT_COLOR))
                .expect("Could not render the font tileset.");
            let mut tileset = HashMap::new();
            for (index, glyph) in game_glyphs.chars().enumerate() {
                let pos = (index as i32 * tile_size_px.x as i32, 0);
                let tile = tiles.subimage(Rectangle::new(pos, tile_size_px));
                tileset.insert(glyph, tile);
            }
            Ok(tileset)
        }));

        Ok(Self {
            game_state: GameState::Playing,
            title,
            mononoki_font_info,
            square_font_info,
            lost_game_message,
            char_map,
            inventory,
            map_size,
            map,
            entities,
            player_id,
            tileset,
            tile_size_px,
        })
    }

    /// Process keyboard and mouse, update the game state
    fn update(&mut self, window: &mut Window) -> Result<()> {

        match self.game_state {
            GameState::Playing => {
                let took_turn = update_player(self, window);

                if took_turn {
                    update_monsters(self, window);
                }

                if window.keyboard()[Key::Escape].is_down() {
                    window.close();
                }

                if self.entities[self.player_id].hp() <= 0 {
                    self.game_state = GameState::Lost;
                }
            },

            GameState::Lost => {
            },
        }

        Ok(())
    }

    /// Draw stuff on the screen
    fn draw(&mut self, window: &mut Window) -> Result<()> {
        window.clear(BACKGROUND_COLOR)?;

        // Draw the game title
        self.title.execute(|image| {
            window.draw(
                &image
                    .area()
                    .with_center((window.screen_size().x as i32 / 2, 40)),
                Img(&image),
            );
            Ok(())
        })?;

        // Draw the mononoki font credits
        self.mononoki_font_info.execute(|image| {
            window.draw(
                &image
                    .area()
                    .translate((2, window.screen_size().y as i32 - 60)),
                Img(&image),
            );
            Ok(())
        })?;

        // Draw the Square font credits
        self.square_font_info.execute(|image| {
            window.draw(
                &image
                    .area()
                    .translate((2, window.screen_size().y as i32 - 30)),
                Img(&image),
            );
            Ok(())
        })?;

        let tile_size_px = self.tile_size_px;
        let offset_px = Vector::new(MAP_DRAW_X_OFFSET as u8, MAP_DRAW_Y_OFFSET as u8);

        // Draw the map
        for tile in self.map.iter() {
            let pos_px = tile.pos.times(tile_size_px);
            let pos = offset_px + pos_px;
            draw_char(&self.char_map, window, pos, tile.glyph);
        }

        // Draw entities
        for entity in self.entities.iter() {
            let pos_px = entity.pos.times(tile_size_px);
            let pos = offset_px + pos_px;
            draw_char(&self.char_map, window, pos, entity.glyph);
        }

        let player = &self.entities[self.player_id];
        let full_health_width_px = 100.0;
        let current_health_width_px =
            (player.hp() as f32 / player.max_hp() as f32) * full_health_width_px;

        let map_size_px = self.map_size.times(tile_size_px);
        let health_bar_pos_px = offset_px + Vector::new(map_size_px.x, 0.0);

        // Full health
        window.draw(
            &Rectangle::new(health_bar_pos_px, (full_health_width_px, tile_size_px.y)),
            Col(Color::RED.with_alpha(0.5)),
        );

        // Current health
        window.draw(
            &Rectangle::new(health_bar_pos_px, (current_health_width_px, tile_size_px.y)),
            Col(Color::RED),
        );

        // Current health
        self.inventory.execute(|image| {
            window.draw(
                &image
                    .area()
                    .translate(health_bar_pos_px + Vector::new(0, tile_size_px.y)),
                Img(&image),
            );
            Ok(())
        })?;

        // Draw Message
        if self.game_state == GameState::Lost {
            self.lost_game_message.execute(|image| {
                window.draw(
                    &image
                        .area()
                        .translate((MAP_DRAW_X_OFFSET as u16 + 100, MAP_DRAW_X_OFFSET as u16 + 120)),
                    Img(&image),
                );
                Ok(())
            })?;
        }

        Ok(())
    }
}

// Update Functions
fn update_monsters(game: &mut Game, window: &mut Window) {
    let player = game.entities[game.player_id].clone();
    // NOTE copies all entities every frame!
    let entities = game.entities.clone();

    let mut attacks: Vec<(EntityId, EntityId)> = Vec::new();

    for (index, monster) in game.entities.iter_mut().filter(|entity| entity.typ.is_monster()).enumerate() {
        let prev_position = monster.pos;
        let pos_diff = player.pos - monster.pos;

        monster.pos += Vector::new(pos_diff.x.signum(), pos_diff.y.signum());
        
        if blocked_tile(monster.pos, &game.map) {
            monster.pos = prev_position;
        } else if let Some(entity) = occupied_tile(monster.pos, &entities) {
            monster.pos = prev_position;
            if entity.typ.is_player() {
                attacks.push((index, entities.iter().enumerate().find(|(index, ent)| **ent == entity).unwrap().0));
            } // else if monster is berserk, attack other monster
        }
    }

    for attack in attacks {
        let typ = &mut game.entities[attack.1].typ;
        match typ {
            EntityType::Player(mut player) => {
                typ.lose_hp(1);
            },

            EntityType::Monster(mut monster) => {
                typ.lose_hp(1);
            },

            _ => { },
        }
    }
}

fn update_player(game: &mut Game, window: &mut Window) -> bool {
    use ButtonState::*;

    let mut took_turn: bool = false;

    let player = &mut game.entities[game.player_id];
    let previous_pos = player.pos;
    if window.keyboard()[Key::Left] == Pressed {
        player.pos.x = clamp(0.0, MAP_WIDTH as f32, player.pos.x - 1.0);
        took_turn = true;
    }
    if window.keyboard()[Key::Right] == Pressed {
        player.pos.x = clamp(0.0, MAP_WIDTH as f32, player.pos.x + 1.0);
        took_turn = true;
    }
    if window.keyboard()[Key::Up] == Pressed {
        player.pos.y = clamp(0.0, MAP_HEIGHT as f32, player.pos.y - 1.0);
        took_turn = true;
    }
    if window.keyboard()[Key::Down] == Pressed {
        player.pos.y = clamp(0.0, MAP_HEIGHT as f32, player.pos.y + 1.0);
        took_turn = true;
    }

    if blocked_tile(player.pos, &game.map) {
        player.pos = previous_pos;
        took_turn = false;
    }

    return took_turn;
}

// Draw Function
fn draw_char(char_map: &HashMap<u32, Image>, window: &mut Window, pos: Vector, chr: char) {
    let char_ix = chr as u32;
    let rect = Rectangle::new(pos, Vector::new(16, 16));
    window.draw(&rect, Img(&char_map[&char_ix]));
}

fn main() {
    // NOTE: Set HIDPI to 1.0 to get pixel-perfect rendering.
    // Otherwise the window resizes to whatever value the OS sets and
    // scales the contents.
    // https://docs.rs/glutin/0.19.0/glutin/dpi/index.html
    std::env::set_var("WINIT_HIDPI_FACTOR", "1.0");

    let settings = Settings {
        // If the graphics do need to be scaled (e.g. using
        // `with_center`), blur them. This looks better with fonts.
        scale: quicksilver::graphics::ImageScaleStrategy::Blur,
        ..Default::default()
    };
    run::<Game>("Ludem Dare 45", Vector::new(800, 600), settings);
}
