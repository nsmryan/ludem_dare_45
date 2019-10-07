use std::collections::HashMap;

use rand::*;
use noise::*;

use quicksilver::prelude::*;


const NUM_LEVEL_GAME: usize = 4;

const BACKGROUND_COLOR: Color = Color::BLACK;
const SCALE: f32 = 2.5;

const WALL_CHAR: char = 2 as char;
const ITERP_TIME: f64 = 0.15;
const DRAWS_PER_IDLE_FRAME: usize = 2;
const DRAWS_PER_ATTACK_FRAME: usize = 1;
const DRAWS_PER_DEATH_FRAME: usize = 2;

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

const MAP_WIDTH: usize = 10;
const MAP_HEIGHT: usize = 10;

const MAP_DRAW_X_OFFSET: usize  = 200;
const MAP_DRAW_Y_OFFSET: usize  = 120;
const TILE_WIDTH_PX: u32 = 40;
const TILE_HEIGHT_PX: u32 = 40;

const MILLIS_PER_UPDATE: f64 = 0.5;
const IDLE_PROB: f32 = 1.0;
const PLAYER_CHARACTER: char = 139 as char;

static RED: Color         = Color { r: 161.0 / 255.0, g: 22.0  / 255.0, b: 52.0  / 255.0, a: 1.0 };
static DARK_GREEN: Color  = Color { r: 25.0  / 255.0, g: 69.0  / 255.0, b: 35.0  / 255.0, a: 1.0 };
static GREEN: Color       = Color { r: 15.0  / 255.0, g: 128.0 / 255.0, b: 55.0  / 255.0, a: 1.0 };
static BRIGHT_BLUE: Color = Color { r: 101.0 / 255.0, g: 233.0 / 255.0, b: 228.0 / 255.0, a: 1.0 };
static DARK_ORANGE: Color = Color { r: 186.0 / 255.0, g: 98.0  / 255.0, b: 20.0  / 255.0, a: 1.0 };
static ORANGE: Color      = Color { r: 255.0 / 255.0, g: 138.0 / 255.0, b: 0.0   / 255.0, a: 1.0 };
static WHITE: Color       = Color { r: 238.0 / 255.0, g: 243.0 / 255.0, b: 244.0 / 255.0, a: 1.0 };
static VERY_GRAY: Color   = Color { r: 29.0  / 255.0, g: 30.0  / 255.0, b: 32.0  / 255.0, a: 1.0 };
static DARK_GRAY: Color   = Color { r: 54.0  / 255.0, g: 56.0  / 255.0, b: 49.0  / 255.0, a: 1.0 };
static LIGHT_GRAY: Color  = Color { r: 76.0  / 255.0, g: 79.0  / 255.0, b: 84.0  / 255.0, a: 1.0 };
static STONE_GRAY: Color  = Color { r: 67.0  / 255.0, g: 59.0  / 255.0, b: 62.0  / 255.0, a: 1.0 };
static LIGHT_BROWN: Color = Color { r: 158.0 / 255.0, g: 134.0 / 255.0, b: 100.0 / 255.0, a: 1.0 };

static MONSTER_COLOR: Color = LIGHT_BROWN;
static TRAP_COLOR: Color = ORANGE;

#[derive(Clone, Debug, PartialEq)]
enum GameState {
    Playing(usize),
    Lost,
    NextLevel(usize),
    Win,
}

#[derive(Clone, Debug, PartialEq)]
struct Tile {
    pos: Vector,
    glyph: char,
    color: Color,
    blocks: bool,
}

impl Tile {
    fn wall(x: usize, y: usize) -> Tile {
        return Tile {
            pos: Vector::new(x as f32, y as f32),
            glyph: 219 as char,
            color: WHITE,
            blocks: false,
        };
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Status {
    Berserk,
}

type Hp = i32;

type EntityId = usize;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Arrow {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Trap {
    Berserk,
    Kill,
    Bump,
    Teleport,
    CountDown(u8),
    Arrow(Arrow),
    NextLevel,
    Win,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum MonsterType {
    Gol,
    Rook,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Monster {
    hp: Hp,
    max_hp: Hp,
    status: Option<Status>,
    typ: MonsterType,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Player {
    hp: Hp,
    max_hp: Hp,
    status: Option<Status>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum EntityType {
    Trap(Trap),
    Monster(Monster),
    Player(Player),
}

impl EntityType {
    fn monster(max_hp: Hp, typ: MonsterType) -> EntityType {
        return EntityType::Monster(Monster {
            hp: max_hp,
            max_hp: max_hp,
            status: None,
            typ: typ,
        });
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

    fn is_rook(&self) -> bool {
        return match self {
            // TODO very fragile way to do this. should carry a monster type
            EntityType::Monster(monster) => monster.typ == MonsterType::Rook,
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

#[derive(Clone, Copy, Debug, PartialEq)]
enum Animation {
    MonsterAttack(MonsterType, Vector, usize),
    MonsterDeath(MonsterType, Vector, usize),
}

fn generate_map(size: Vector) -> Vec<Tile> {
    let width = size.x as usize;
    let height = size.y as usize;
    let mut map = Vec::with_capacity(width * height);
    for x in 0..width {
        for y in 0..height {
            let mut tile = Tile::wall(x, y);

            if x == 0 || x == width - 1 || y == 0 || y == height - 1 {
                tile.glyph = WALL_CHAR;
                tile.blocks = true;
            };
            map.push(tile);
        }
    }

    let mut rng = thread_rng();
    let mut walls_placed = 0;
    while walls_placed < 5 {
        let mut x = rng.gen_range(2 as i32, MAP_WIDTH as i32);
        let mut y = rng.gen_range(2 as i32, MAP_HEIGHT as i32);
        let x_dir: i32 = rng.gen_range(-1, 2);
        let y_dir: i32 = rng.gen_range(-1, 2);
        let dist = rng.gen_range(1, 5);

        if x_dir.abs() == y_dir.abs() {
            continue;
        }

        for _square_index in 0..dist {
            if let Some(map_index) = map.iter().position(|tile| tile.pos.x == x as f32 && tile.pos.y == y as f32) {
                map[map_index] = Tile::wall(x as usize, y as usize);
                map[map_index].glyph = WALL_CHAR;
                map[map_index].blocks = true;
                x += x_dir;
                y += y_dir;
                if x < 0 || x >= MAP_WIDTH as i32 || y < 0 || y > MAP_HEIGHT as i32 {
                    break;
                }
            }
        }

        if rng.gen_range(0.0, 1.0) > 0.5 {
            let x_dir = x_dir * -1;
            let y_dir = y_dir * -1;
            x += x_dir;
            y += y_dir;
            if let Some(map_index) = map.iter().position(|tile| tile.pos.x == x as f32 && tile.pos.y == y as f32) {
                map[map_index] = Tile::wall(x as usize, y as usize);
                map[map_index].glyph = WALL_CHAR;
                map[map_index].blocks = true;
            }
        }

        walls_placed += 1;
    }

    return map;
}

fn blocked_tile(pos: Vector, map: &Map) -> bool {
    return map.iter().any(|tile| tile.blocks && tile.pos == pos);
}

fn occupied_tile(pos: Vector, entities: &Vec<Entity>) -> Option<Entity> {
    return entities.iter().find(|entity| entity.pos == pos).map(|entity| entity.clone());
}

fn trap_tile(pos: Vector, entities: &Vec<Entity>) -> Option<Entity> {
    return entities.iter().find(|entity| entity.typ.is_trap() && entity.pos == pos).map(|entity| entity.clone());
}

fn magnitude(vec: Vector) -> f32 {
    return (vec.x.powi(2) + vec.y.powi(2)).sqrt();
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
enum AnimState {
    None,
    Idle(usize),
    Attacking(usize, Arrow),
}

impl AnimState {
    fn is_none(&self) -> bool {
        match self {
            AnimState::None => true,
            AnimState::Idle(_) => false,
            AnimState::Attacking(_, _) => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Entity {
    last_pos: Vector,
    pos: Vector,
    glyph: char,
    color: Color,
    typ: EntityType,
    anim_state: AnimState,
}

impl Entity {
    fn trap(pos: Vector, trap: Trap) -> Entity {
        let color = match trap {
            Trap::NextLevel => WHITE,
            Trap::Win => WHITE,
            _ => TRAP_COLOR,
        };

        let chr = match trap {
            Trap::Kill => 147 as char,
            Trap::Berserk => '*',
            Trap::Bump => 42 as char,
            Trap::Teleport => '!',
            Trap::CountDown(n) => ('0' as u8 + n) as char,
            Trap::Arrow(dir) => {
                match dir {
                    Arrow::Left => 17 as char,
                    Arrow::Right => 16 as char,
                    Arrow::Up => 18 as char,
                    Arrow::Down => 19 as char,
                }
            }
            Trap::NextLevel => 3 as char,
            Trap::Win => 255 as char,
        };

        let anim_state =
            match trap {
                Trap::Arrow(_) | Trap::NextLevel | Trap::Berserk | Trap::CountDown(_) => AnimState::None,
                _ => AnimState::Idle(0),
            };

        Entity {
            last_pos: pos,
            pos: pos,
            glyph: chr,
            color: color,
            typ: EntityType::trap(trap),
            anim_state: anim_state,
        }
    }

    fn gol(pos: Vector) -> Entity {
        Entity {
            last_pos: pos,
            pos: pos,
            glyph: 152 as char,
            color: MONSTER_COLOR,
            typ: EntityType::monster(1, MonsterType::Gol),
            anim_state: AnimState::Idle(0),
        }
    }

    fn rook(pos: Vector) -> Entity {
        Entity {
            last_pos: pos,
            pos: pos,
            glyph: 130 as char,
            color: MONSTER_COLOR,
            typ: EntityType::monster(2, MonsterType::Rook),
            anim_state: AnimState::Idle(0),
        }
    }
}

fn map_pos<R: Rng>(rng: &mut R) -> Vector {
    return Vector::new(rng.gen_range(1, MAP_WIDTH as u16 - 1),
                       rng.gen_range(1, MAP_HEIGHT as u16 - 1));
}

fn map_unique_pos(map: Map) -> impl Iterator<Item=Vector> {
    let mut rng = thread_rng();
    let mut positions: Vec<Vector> = Vec::new();
    return std::iter::from_fn(move || {
        let mut new_pos = map_pos(&mut rng);
        while positions.iter().find(|pos| **pos == new_pos).is_some() ||
              map[new_pos.y as usize + new_pos.x as usize * MAP_HEIGHT].blocks {
            new_pos = map_pos(&mut rng);
        }

        positions.push(new_pos);

        return Some(new_pos);
    });
}

fn generate_entities(last_level: bool, entities: &mut Vec<Entity>, map: &Map) -> Vector {
    let player_pos;

    if false {
        entities.push(Entity::gol(Vector::new(4, 4)));
        entities.push(Entity::rook(Vector::new(2, 1)));
        entities.push(Entity::trap(Vector::new(6, 6), Trap::Bump)); 
        entities.push(Entity::trap(Vector::new(3, 8), Trap::Berserk)); 
        entities.push(Entity::trap(Vector::new(4, 2), Trap::Arrow(Arrow::Left))); 
        entities.push(Entity::trap(Vector::new(4, 3), Trap::Arrow(Arrow::Left))); 
        entities.push(Entity::trap(Vector::new(4, 4), Trap::Arrow(Arrow::Left))); 
        entities.push(Entity::trap(Vector::new(7, 6), Trap::Kill)); 
        entities.push(Entity::trap(Vector::new(7, 7), Trap::Kill)); 
        entities.push(Entity::trap(Vector::new(7, 2), Trap::Teleport)); 
        entities.push(Entity::trap(Vector::new(4, 8), Trap::Teleport)); 
        entities.push(Entity::trap(Vector::new(1, 2), Trap::CountDown(3))); 
        entities.push(Entity::trap(Vector::new(8, 8), Trap::NextLevel));

        player_pos = Vector::new(3, 4);
    } else {
        let mut positions = map_unique_pos(map.clone());

        entities.push(Entity::gol(positions.next().unwrap()));
        entities.push(Entity::rook(positions.next().unwrap()));
        entities.push(Entity::trap(positions.next().unwrap(), Trap::Bump));
        entities.push(Entity::trap(positions.next().unwrap(), Trap::Kill));
        entities.push(Entity::trap(positions.next().unwrap(), Trap::Kill));
        entities.push(Entity::trap(positions.next().unwrap(), Trap::Kill));
        entities.push(Entity::trap(positions.next().unwrap(), Trap::Kill));
        entities.push(Entity::trap(positions.next().unwrap(), Trap::Arrow(Arrow::Left)));
        entities.push(Entity::trap(positions.next().unwrap(), Trap::Arrow(Arrow::Right)));
        entities.push(Entity::trap(positions.next().unwrap(), Trap::Arrow(Arrow::Up)));
        entities.push(Entity::trap(positions.next().unwrap(), Trap::Arrow(Arrow::Down)));

        if last_level {
            entities.push(Entity::trap(positions.next().unwrap(), Trap::Win));
        } else {
            entities.push(Entity::trap(positions.next().unwrap(), Trap::NextLevel));
        }

        player_pos = positions.next().unwrap();
    }

    return player_pos;
}

struct Game {
    game_state: GameState,
    title: Asset<Image>,
    mononoki_font_info: Asset<Image>,
    square_font_info: Asset<Image>,
    lost_game_message: Asset<Image>,
    win_game_message: Asset<Image>,
    char_map: Asset<HashMap<u32, Image>>,
    inventory: Asset<Image>,
    map_size: Vector,
    map: Map,
    entities: Vec<Entity>,
    player_id: usize,
    tileset: Asset<HashMap<char, Image>>,
    noise: Perlin,
    gol_idle: Asset<Vec<Image>>,
    rook_idle: Asset<Vec<Image>>,
    player_idle: Asset<Vec<Image>>,
    gol_attack_up: Asset<Vec<Image>>,
    gol_attack_down: Asset<Vec<Image>>,
    gol_attack_left: Asset<Vec<Image>>,
    gol_attack_right: Asset<Vec<Image>>,
    rook_attack_down: Asset<Vec<Image>>,
    rook_attack_up: Asset<Vec<Image>>,
    rook_attack_right: Asset<Vec<Image>>,
    rook_attack_left: Asset<Vec<Image>>,
    trap_win: Asset<Vec<Image>>,
    trap_damage: Asset<Vec<Image>>,
    trap_arrow_up: Asset<Vec<Image>>,
    trap_arrow_down: Asset<Vec<Image>>,
    trap_arrow_right: Asset<Vec<Image>>,
    trap_arrow_left: Asset<Vec<Image>>,
    trap_random_direction: Asset<Vec<Image>>,
    trap_tele: Asset<Vec<Image>>,
    gol_death: Asset<Vec<Image>>,
    rook_death: Asset<Vec<Image>>,
    time_passed: f64,
    animations: Vec<Animation>,
}

impl State for Game {
    /// Load the assets and initialise the game
    fn new() -> Result<Self> {
        // The Mononoki font: https://madmalik.github.io/mononoki/
        // License: SIL Open Font License 1.1
        let font_mononoki = "mononoki-Regular.ttf";

        let title = Asset::new(Font::load(font_mononoki).and_then(|font| {
            font.render("StoneFall", &FontStyle::new(72.0, WHITE))
        }));

        let font_image = "LD45_SpriteSheet.png";
        let char_map = Asset::new(Image::load(font_image).and_then(|image| {
            let mut char_map = HashMap::new();
            let char_size = Vector::new(16, 16);
            for char_ix in 0..256 {
                let char_x = char_ix % 16;
                let char_y = char_ix / 16;
                let char_pos = Vector::new(char_x * 16, char_y * 16);
                char_map.insert(char_ix,
                                image.subimage(Rectangle::new(char_pos, char_size)));
            }

            return Ok(char_map);
        }));

        let gol_idle_name = "Gol_Idle.png";
        let gol_idle = Asset::new(Image::load(gol_idle_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut gol_idle = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                gol_idle.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(gol_idle);
        }));

        let rook_idle_name = "Rook_Idle.png";
        let rook_idle = Asset::new(Image::load(rook_idle_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut rook_idle = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                rook_idle.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(rook_idle);
        }));

        let gol_attack_name = "Gol_AttackUp.png";
        let gol_attack_up = Asset::new(Image::load(gol_attack_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut gol_attack = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                gol_attack.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(gol_attack);
        }));

        let gol_attack_name = "Gol_AttackDown.png";
        let gol_attack_down = Asset::new(Image::load(gol_attack_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut gol_attack = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                gol_attack.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(gol_attack);
        }));

        let gol_attack_name = "Gol_AttackRight.png";
        let gol_attack_right = Asset::new(Image::load(gol_attack_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut gol_attack = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                gol_attack.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(gol_attack);
        }));

        let gol_attack_name = "Gol_AttackLeft.png";
        let gol_attack_left = Asset::new(Image::load(gol_attack_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut gol_attack = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                gol_attack.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(gol_attack);
        }));

        let rook_attack_name = "Rook_AttackUp.png";
        let rook_attack_up = Asset::new(Image::load(rook_attack_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut rook_attack = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                rook_attack.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(rook_attack);
        }));

        let rook_attack_name = "Rook_AttackDown.png";
        let rook_attack_down = Asset::new(Image::load(rook_attack_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut rook_attack = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                rook_attack.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(rook_attack);
        }));

        let rook_attack_name = "Rook_AttackRight.png";
        let rook_attack_right = Asset::new(Image::load(rook_attack_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut rook_attack = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                rook_attack.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(rook_attack);
        }));

        let rook_attack_name = "Rook_AttackLeft.png";
        let rook_attack_left = Asset::new(Image::load(rook_attack_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut rook_attack = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                rook_attack.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(rook_attack);
        }));

        let player_idle_name = "Player_Idle.png";
        let player_idle = Asset::new(Image::load(player_idle_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut player_idle = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                player_idle.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(player_idle);
        }));

        let gol_death_name = "Gol_Die.png";
        let gol_death = Asset::new(Image::load(gol_death_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut gol_death = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                gol_death.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(gol_death);
        }));

        let trap_win_name = "McMuffin.png";
        let trap_win = Asset::new(Image::load(trap_win_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut trap_win = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                trap_win.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(trap_win);
        }));

        let trap_damage_name = "DamageTrap.png";
        let trap_damage = Asset::new(Image::load(trap_damage_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut trap_damage = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                trap_damage.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(trap_damage);
        }));

        let trap_arrow_up_name = "DirectionPushTrapUp.png";
        let trap_arrow_up = Asset::new(Image::load(trap_arrow_up_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut trap_arrow_up = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                trap_arrow_up.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(trap_arrow_up);
        }));

        let trap_arrow_down_name = "DirectionPushTrapDown.png";
        let trap_arrow_down = Asset::new(Image::load(trap_arrow_down_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut trap_arrow_down = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                trap_arrow_down.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(trap_arrow_down);
        }));

        let trap_arrow_right_name = "DirectionPushTrapRight.png";
        let trap_arrow_right = Asset::new(Image::load(trap_arrow_right_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut trap_arrow_right = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                trap_arrow_right.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(trap_arrow_right);
        }));

        let trap_arrow_left_name = "DirectionPushTrapLeft.png";
        let trap_arrow_left = Asset::new(Image::load(trap_arrow_left_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut trap_arrow_left = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                trap_arrow_left.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(trap_arrow_left);
        }));

        let trap_random_direction_name = "RandomDirectionTrap.png";
        let trap_random_direction = Asset::new(Image::load(trap_random_direction_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut trap_random_direction = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                trap_random_direction.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(trap_random_direction);
        }));

        let trap_tele_name = "TeleTrap.png";
        let trap_tele = Asset::new(Image::load(trap_tele_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut trap_tele = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                trap_tele.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(trap_tele);
        }));

        let rook_death_name = "Rook_Die.png";
        let rook_death = Asset::new(Image::load(rook_death_name).and_then(|image| {
            let num_sprites: u32 = image.area().size().x as u32 / 16;
            let mut rook_death = Vec::new();
            let anim_size = Vector::new(16, 16);
            for image_index in 0..num_sprites {
                let pos = Vector::new(image_index * 16, 0);
                rook_death.push(image.subimage(Rectangle::new(pos, anim_size)));
            }

            return Ok(rook_death);
        }));

        let lost_game_message = Asset::new(Font::load(font_mononoki).and_then(|font| {
            font.render("You Lose!", &FontStyle::new(72.0, WHITE))
        }));

        let win_game_message = Asset::new(Font::load(font_mononoki).and_then(|font| {
            font.render("You Win!", &FontStyle::new(72.0, WHITE))
        }));

        let mononoki_font_info = Asset::new(Font::load(font_mononoki).and_then(|font| {
            font.render(
                "",
                &FontStyle::new(20.0, WHITE),
            )
        }));

        let square_font_info = Asset::new(Font::load(font_mononoki).and_then(move |font| {
            font.render(
                "A Ludum Dare Game by Joel and Noah Ryan",
                &FontStyle::new(20.0, WHITE),
            )
        }));

        // TODO inventory message is here.
        let inventory = Asset::new(Font::load(font_mononoki).and_then(move |font| {
            font.render(
                "",
                &FontStyle::new(20.0, WHITE),
            )
        }));

        let map_size = Vector::new(MAP_WIDTH as u8, MAP_HEIGHT as u8);
        let mut map = generate_map(map_size);
        let player_id = 0;

        let player_start = Vector::new(5, 3);
        let mut entities = Vec::new();
        entities.push(Entity {
            last_pos: player_start,
            pos: Vector::new(5, 3),
            glyph: PLAYER_CHARACTER,
            color: WHITE,
            typ: EntityType::Player(Player { 
                hp: 5,
                max_hp: 5,
                status: None,
            }),
            anim_state: AnimState::Idle(0),
        });
        map[player_start.y as usize + player_start.x as usize * MAP_HEIGHT] =
            Tile::wall(player_start.x as usize, player_start.y as usize);
        let player_pos = generate_entities(false, &mut entities, &map);
        entities[0].pos = player_pos;

        for tile in map.iter_mut() {
            if occupied_tile(tile.pos, &entities).is_some() {
                *tile = Tile::wall(tile.pos.x as usize, tile.pos.y as usize);
            }
        }

        // The Square font: http://strlen.com/square/?s[]=font
        // License: CC BY 3.0 https://creativecommons.org/licenses/by/3.0/deed.en_US
        let font_square = "square.ttf";
        let game_glyphs = "#@g.%";
        let tile_size_px = Vector::new(TILE_WIDTH_PX, TILE_HEIGHT_PX);
        let tileset = Asset::new(Font::load(font_square).and_then(move |text| {
            let tiles = text
                .render(game_glyphs, &FontStyle::new(tile_size_px.y, WHITE))
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
            game_state: GameState::Playing(0),
            title,
            mononoki_font_info,
            square_font_info,
            lost_game_message,
            win_game_message,
            char_map,
            inventory,
            map_size,
            map,
            entities,
            player_id,
            tileset,
            noise: Perlin::new(),
            gol_idle,
            rook_idle,
            player_idle,
            time_passed: 0.0,
            animations: Vec::new(),
            gol_attack_right,
            gol_attack_up,
            gol_attack_down,
            gol_attack_left,
            rook_attack_up,
            rook_attack_down,
            rook_attack_right,
            rook_attack_left,
            gol_death,
            rook_death,
            trap_win,
            trap_damage,
            trap_arrow_up,
            trap_arrow_down,
            trap_arrow_right,
            trap_arrow_left,
            trap_random_direction,
            trap_tele,
        })
    }

    /// Process keyboard and mouse, update the game state
    fn update(&mut self, window: &mut Window) -> Result<()> {

        match self.game_state {
            GameState::Win => {
                let stairs_pos = self.entities.iter().find(|ent| {
                    match ent.typ {
                        EntityType::Trap(Trap::Win) => true,
                        _ => false,
                    }
                }).unwrap().pos;
                self.entities[0].pos = stairs_pos;
            }

            GameState::NextLevel(n) => {
                if n >= NUM_LEVEL_GAME {
                    self.game_state = GameState::Win;
                } else {
                    let map_size = Vector::new(MAP_WIDTH as u8, MAP_HEIGHT as u8);
                    let mut map = generate_map(map_size);
                    self.map = generate_map(map_size);

                    let player = self.entities[0].clone();
                    self.entities.clear();
                    self.entities.push(player);
                    let player_pos = generate_entities(n + 1 == NUM_LEVEL_GAME, &mut self.entities, &self.map);
                    self.entities[0].pos = player_pos;

                    self.game_state = GameState::Playing(n + 1);
                }
            }

            GameState::Playing(n) => {
                let took_turn = update_player(self, window);

                self.time_passed += MILLIS_PER_UPDATE / 1000.0;
                if took_turn {
                    self.time_passed = 0.0;

                    let mut win = false;
                    let mut next_level = false;

                    update_monsters(self, window);
                    resolve_traps(&mut self.entities, &self.map, &mut self.animations, &mut next_level, &mut win);

                    if next_level {
                        self.game_state = GameState::NextLevel(n);
                    } else if win {
                        self.game_state = GameState::Win;
                    }
                }

                if window.keyboard()[Key::Escape].is_down() {
                    window.close();
                }

                if self.entities[self.player_id].hp() <= 0 {
                    self.game_state = GameState::Lost;
                }

                self.entities = self.entities.iter().filter(|entity| {
                    if entity.typ.is_monster() {
                        return entity.hp() > 0;
                    }

                    return true;
                }).map(|ent| ent.clone()).collect();
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

        let tile_size_px = Vector::new(TILE_WIDTH_PX, TILE_HEIGHT_PX);
        let offset_px = Vector::new(MAP_DRAW_X_OFFSET as u8, MAP_DRAW_Y_OFFSET as u8);

        // draw map
        for tile in self.map.iter() {
            draw_tile(tile, window, offset_px, &mut self.char_map, &mut self.noise);
        }

        // draw entities
        // draw traps
        for entity in self.entities.iter() {
            if entity.typ.is_trap() && entity.anim_state.is_none() {
                let tile_size_px = Vector::new(TILE_WIDTH_PX, TILE_HEIGHT_PX);
                let pos_px = entity.pos.times(tile_size_px);
                let pos = offset_px + pos_px;
                draw_entity(entity, pos, window, &mut self.char_map);
            }
        }

        // draw other entities
        for entity in self.entities.iter_mut() {
            let tile_size_px = Vector::new(TILE_WIDTH_PX, TILE_HEIGHT_PX);
            let ent_pos = entity.pos;
            let last_ent_pos = entity.last_pos;
            let lerp_amount = clamp(0.0, ITERP_TIME as f32, self.time_passed as f32) / ITERP_TIME as f32;
            let ent_pos = Vector::new(lerp(last_ent_pos.x, ent_pos.x, lerp_amount),
                                      lerp(last_ent_pos.y, ent_pos.y, lerp_amount));
            if magnitude(ent_pos - entity.pos) < 0.01 {
                entity.last_pos = entity.pos;
            }
            let pos_px = ent_pos.times(tile_size_px);
            let pos = offset_px + pos_px;

            match entity.anim_state {
                AnimState::None => {
                    draw_entity(entity, pos, window, &mut self.char_map);
                }

                AnimState::Attacking(index, direction) => {
                    match entity.typ {
                        EntityType::Monster(_) => {
                            let anims;
                            if entity.typ.is_rook() {
                                anims = &mut self.rook_attack_right;
                            } else {
                                match direction {
                                    Arrow::Up => anims = &mut self.gol_attack_up,
                                    Arrow::Down => anims = &mut self.gol_attack_down,
                                    Arrow::Right => anims = &mut self.gol_attack_right,
                                    Arrow::Left => anims = &mut self.gol_attack_left,
                                }
                            }
                            anims.execute(|anims| {
                                let rect = Rectangle::new(pos,
                                                          Vector::new(16, 16));
                                let anim_index = index / DRAWS_PER_ATTACK_FRAME;
                                window.draw_ex(&rect,
                                               Blended(&anims[anim_index], entity.color),
                                               Transform::scale(Vector::new(SCALE, SCALE)),
                                               SCALE);
                                if (index + 1) >= (anims.len() * DRAWS_PER_ATTACK_FRAME) {
                                    entity.anim_state = AnimState::Idle(0);
                                } else {
                                    entity.anim_state = AnimState::Attacking(index + 1, direction);
                                }
                                return Ok(());
                            }).unwrap();
                        },

                        _ => continue,
                    }
                }

                AnimState::Idle(index) => {
                    match entity.typ {
                        EntityType::Monster(_) | EntityType::Player(_) | EntityType::Trap(_) => {
                            let idle_anims;
                            if entity.typ.is_player() {
                                idle_anims = &mut self.player_idle;
                            } else if entity.typ.is_monster() {
                                if entity.typ.is_rook() {
                                idle_anims = &mut self.rook_idle;
                                } else {
                                    idle_anims = &mut self.gol_idle;
                                }
                            } else if entity.typ.is_trap() {
                                match entity.typ {
                                    EntityType::Trap(trap) => {
                                        match trap {
                                            Trap::Kill => idle_anims = &mut self.trap_damage,
                                            Trap::Bump => idle_anims = &mut self.trap_random_direction,
                                            Trap::Teleport => idle_anims = &mut self.trap_tele,
                                            Trap::Win => idle_anims = &mut self.trap_win,
                                            Trap::Arrow(arrow) => {
                                                match arrow {
                                                    Arrow::Up => idle_anims = &mut self.trap_arrow_up,
                                                    Arrow::Down => idle_anims = &mut self.trap_arrow_down,
                                                    Arrow::Right => idle_anims = &mut self.trap_arrow_right,
                                                    Arrow::Left => idle_anims = &mut self.trap_arrow_left,
                                                }
                                            },
                                            
                                            _ => panic!("Should not be here..."),
                                        }
                                    },

                                    _ => panic!("Unreachable type!"),
                                }
                            } else {
                                panic!("Unreachable arm for animation!");
                            }

                            idle_anims.execute(|idle_anims| {
                                let rect = Rectangle::new(pos,
                                                          Vector::new(16, 16));
                                let anim_index = index / DRAWS_PER_IDLE_FRAME;
                                window.draw_ex(&rect,
                                               Blended(&idle_anims[anim_index], entity.color),
                                               Transform::scale(Vector::new(SCALE, SCALE)),
                                               SCALE);
                                if (index + 1) >= (idle_anims.len() * DRAWS_PER_IDLE_FRAME) {
                                    entity.anim_state = AnimState::Idle(0);
                                } else {
                                    entity.anim_state = AnimState::Idle(index + 1);
                                }
                                return Ok(());
                            }).unwrap();
                        },
                    }
                }
            }
        }

        // draw animations
        let mut animations_done = Vec::new();
        for (animation_index, animation) in self.animations.iter_mut().enumerate() {
            match animation {
                Animation::MonsterDeath(monster_typ, loc, sprite_index) => {
                    let anims;
                    match monster_typ {
                        MonsterType::Gol => {
                            anims = &mut self.gol_death;
                        },

                        MonsterType::Rook => {
                            anims = &mut self.rook_death;
                        },
                    }

                    let tile_size_px = Vector::new(TILE_WIDTH_PX, TILE_HEIGHT_PX);
                    let pos_px = loc.times(tile_size_px);
                    let pos = offset_px + pos_px;

                    let mut anim_len = 0;
                    anims.execute(|anims| {
                        let rect = Rectangle::new(pos,
                                                  Vector::new(16, 16));
                        let anim_index = *sprite_index / DRAWS_PER_DEATH_FRAME;
                        window.draw_ex(&rect,
                                       Blended(&anims[anim_index], MONSTER_COLOR),
                                       Transform::scale(Vector::new(SCALE, SCALE)),
                                       SCALE);
                        anim_len = anims.len();
                        return Ok(());
                    }).unwrap();

                    if (*sprite_index + 1) >= (anim_len * DRAWS_PER_DEATH_FRAME) {
                        animations_done.push(animation_index);
                    } else {
                        *animation = Animation::MonsterDeath(monster_typ.clone(), *loc, *sprite_index + 1);
                    }
                }

                Animation::MonsterAttack(monster_typ, loc, sprite_index) => {
                    let tile = self.map.iter().find(|other_tile| other_tile.pos == *loc).unwrap(); //[loc.y as usize + loc.x as usize * MAP_HEIGHT];
                    draw_tile(tile, window, offset_px, &mut self.char_map, &mut self.noise);
                    //let sprites;
                    match monster_typ {
                        MonsterType::Gol => {
                            //sprites = game.gol_attack;
                        }

                        MonsterType::Rook => {
                            //sprites = game.rook_attack;
                        }
                    }
                }
            }
        }

        // TODO remove used animation

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
                        .translate((MAP_DRAW_X_OFFSET as u16 + 40, WINDOW_HEIGHT as u16 - 100)),
                    Img(&image),
                );
                Ok(())
            })?;
        }

        if self.game_state == GameState::Win {
            self.win_game_message.execute(|image| {
                window.draw(
                    &image
                        .area()
                        .translate((MAP_DRAW_X_OFFSET as u16 + 40, WINDOW_HEIGHT as u16 - 100)),
                    Img(&image),
                );
                Ok(())
            })?;
        }

        //let mut rng = thread_rng();
        //for entity in self.entities.iter_mut() {
        //    if (entity.typ.is_monster() || entity.typ.is_player()) &&
        //        entity.idle == None &&
        //        rng.gen_range(0.0, 1.0) < IDLE_PROB {

        //        entity.idle = AnimState::Idle(0);
        //    }
        //}

        Ok(())
    }
}

fn direction(dir: Vector) -> Arrow {
    if dir.x > 0.0 && dir.y == 0.0 {
        Arrow::Right
    } else if dir.x == 0.0 && dir.y < 0.0 {
        Arrow::Up
    } else if dir.x < 0.0 {
        Arrow::Left
    } else { // if dir.y < 0 {
        Arrow::Down
    }
}

// Update Functions
fn update_monsters(game: &mut Game, _window: &mut Window) {
    let player = game.entities[game.player_id].clone();
    // NOTE copies all entities every frame!
    let entities = game.entities.clone();

    let mut attacks: Vec<(EntityId, EntityId)> = Vec::new();

    // For each monster
    for (index, monster) in game.entities.iter_mut().enumerate().filter(|(_index, entity)| entity.typ.is_monster()) {
        let prev_position = monster.pos;

        let pos_diff = player.pos - monster.pos;
        let mut pos_move = monster.pos;
        pos_move.x += pos_diff.x.abs().signum() * pos_diff.x.signum();
        pos_move.y += pos_diff.y.abs().signum() * pos_diff.y.signum();
        // attempt to constrain rooks to lane movement.
        if monster.typ.is_rook() && pos_move.x.abs() == pos_move.y.abs() {
            if pos_diff.x.abs() > pos_diff.y.abs() && !blocked_tile(pos_move, &game.map) {
                pos_move.y = 0.0;
            } else {
                pos_move.x = 0.0;
            }
        }

        if blocked_tile(pos_move, &game.map) {
            pos_move = prev_position;
        } else if let Some(entity) = occupied_tile(pos_move, &entities) {
            if entity.typ.is_player() {
                let dir = direction(pos_move - prev_position);
                pos_move = prev_position;
                //attacks.push((index, entities.iter().enumerate().find(|(_index, ent)| **ent == entity).unwrap().0));
                attacks.push((index, game.player_id));
                // TODO choose arrow based on attack direction...
                monster.anim_state = AnimState::Attacking(0, dir);
            }  else if entity.typ.is_monster() {
                // TODO add for monsters too...
                pos_move = prev_position;
            }
        }

        monster.pos = pos_move;
    }

    // resolve attacks that occured
    for attack in attacks.iter() {
        let typ = &mut game.entities[attack.1].typ;
        match typ {
            EntityType::Player(_player) => {
                typ.lose_hp(1);
            },

            EntityType::Monster(_monster) => {
                typ.lose_hp(1);
            },

            _ => { },
        }

        // only monsters have attack animations
        match game.entities[attack.0].typ {
            EntityType::Monster(_monster) => {
                let pos = game.entities[attack.0].pos;
                //let anim = Animation::MonsterAttack(monster.typ.clone(), pos, 0);
                // TODO removes this kind of animation for attacks...
                //game.animations.push(anim);
            },
            
            _ => (),
        }
    }

    let remove_indices: Vec<usize> =
        game.entities.iter()
                     .enumerate()
                     .filter(|(_ix, ent)| ent.typ.is_monster() && ent.hp() <= 0)
                     .map(|(ix, _ent)| ix)
                     .collect();
    for ix in remove_indices {
        dbg!();
        game.entities.swap_remove(ix);
    }

    // check for idle animations
    //let mut rng = thread_rng();
    //for entity in game.entities.iter_mut() {
    //    if entity.typ.is_monster() &&
    //        entity.idle == None &&
    //        rng.gen_range(0.0, 1.0) < IDLE_PROB {

    //        entity.idle = Some(0);
    //    }
    //}
}

fn lerp_color(src: Color, dst: Color, amount: f32) -> Color {
    return Color {
        r: lerp(src.r, dst.r, amount),
        g: lerp(src.g, dst.g, amount),
        b: lerp(src.b, dst.b, amount),
        a: lerp(src.a, dst.a, amount),
    };
}

fn attempt_move(pos: Vector, offset: Vector, map: &Map) -> Vector {
    let mut new_pos = pos + offset;

    if blocked_tile(new_pos, map) {
        new_pos = pos;
    }

    return new_pos;
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

fn resolve_traps(entities: &mut Vec<Entity>, map: &Map, animations: &mut Vec<Animation>, next_level: &mut bool, win: &mut bool) {
    let mut rng = thread_rng();
    let entities_clone = entities.clone();
    let mut removals: Vec<usize> = Vec::new();
    let mut moves: Vec<(Vector, usize)> = Vec::new();
    let mut count_downs: Vec<(usize, u8)> = Vec::new();

    let trap_iter =
        entities.iter_mut()
                .enumerate()
                .filter(|(_index, ent)| ent.typ.is_player() || ent.typ.is_monster());
    for (index, entity) in trap_iter {
        if let Some(trap_entity) = trap_tile(entity.pos, &entities_clone) {
            let trap_index = entities_clone.iter().position(|other| *other == trap_entity).unwrap();
            match trap_entity.typ {
                EntityType::Trap(trap) => {
                    match trap {
                        Trap::Berserk => {
                            match entity.typ {
                                EntityType::Monster(mut monster) => {
                                    // TODO try setting entity
                                    monster.status = Some(Status::Berserk);
                                },

                                EntityType::Player(mut player) => {
                                    // TODO try setting entity
                                    player.status = Some(Status::Berserk);
                                },

                                _ => panic!("Unexpected entity type!"),
                            }
                        },

                        Trap::Kill => {
                            entity.typ.lose_hp(5);
                            removals.push(trap_index);
                            match entity.typ {
                                EntityType::Monster(monster) => {
                                    animations.push(Animation::MonsterDeath(monster.typ,
                                                                            entity.pos,
                                                                            0));
                                }

                                _ => (),
                            }
                        },

                        Trap::Teleport => {
                            // find next teleport. if one is find, move character there.
                            let entities_len = entities_clone.len();
                            for other_index in 0..entities_len {
                                let offset_index = (other_index + trap_index + 1) % entities_len;
                                let other_entity = &entities_clone[offset_index];
                                match other_entity.typ {
                                    EntityType::Trap(Trap::Teleport) => {
                                        entity.pos = other_entity.pos;
                                        break;
                                    },

                                    _ => { },
                                }
                            }
                        },

                        Trap::Bump => {
                            let pos = entity.pos;
                            let x_offset = rng.gen_range(-1, 2);
                            let y_offset = rng.gen_range(-1, 2);
                            entity.pos =
                                attempt_move(pos,
                                             Vector::new(x_offset, y_offset),
                                             &map);
                        }

                        Trap::CountDown(n) => {
                            if n == 0 {
                                entity.typ.lose_hp(5);
                            } else {
                                count_downs.push((trap_index, n - 1));
                            }
                        },

                        Trap::NextLevel => {
                            dbg!();
                            if entity.typ.is_player() {
                                *next_level = true;
                            }
                        }

                        Trap::Win => {
                            if entity.typ.is_player() {
                                *win = true;
                            }
                        }

                        Trap::Arrow(dir) => {
                            let x_dir;
                            let y_dir;
                            match dir {
                                Arrow::Left => {
                                    x_dir = -1;
                                    y_dir = 0;
                                },

                                Arrow::Right => {
                                    x_dir = 1;
                                    y_dir = 0;
                                },

                                Arrow::Up => {
                                    x_dir = 0;
                                    y_dir = -1;
                                },

                                Arrow::Down => {
                                    x_dir = 0;
                                    y_dir = 1;
                                },
                            }

                            let mut cur_pos = entity.pos;
                            let mut prev_pos = entity.pos;
                            cur_pos += Vector::new(x_dir, y_dir);
                            while !blocked_tile(cur_pos, map) &&
                                  (occupied_tile(cur_pos, &entities_clone) == None ||
                                   occupied_tile(cur_pos, &entities_clone).map(|ent| ent.typ.is_trap()).unwrap_or(false)) {
                                prev_pos = cur_pos;
                                cur_pos += Vector::new(x_dir, y_dir);
                            }
                            moves.push((prev_pos, index));
                        }
                    }
                },

                _ => panic!("Unreachable?"),
            }
        }
    }


    for (pos, index) in moves {
        entities[index].pos = pos;
    }

    for (ix, new_n) in count_downs.iter() {
        entities[*ix].typ = EntityType::Trap(Trap::CountDown(*new_n));
        entities[*ix].glyph = ('0' as u8 + *new_n) as char;
    }

    removals.sort();
    removals.reverse();
    for index in removals.iter() {
        match entities[*index].typ {
            EntityType::Monster(monster) => {
                dbg!();
                animations.push(Animation::MonsterDeath(monster.typ,
                                                        entities[*index].pos,
                                                        0));
            }

            _ => (),
        }
        entities.swap_remove(*index);
    }
}

fn draw_tile(tile: &Tile, window: &mut Window, offset_px: Vector, char_map: &mut Asset<HashMap<u32, Image>>, noise: &mut Perlin) {
    let tile_size_px = Vector::new(TILE_WIDTH_PX, TILE_HEIGHT_PX);
    let pos_px = tile.pos.times(tile_size_px);
    let pos = offset_px + pos_px;
    let color_noise =
        noise.get([6.0 * (pos.x as f64 / WINDOW_WIDTH as f64),
                   6.0 * (pos.y as f64 / WINDOW_HEIGHT as f64)]);
    let mut tile_color = lerp_color(DARK_GRAY, LIGHT_GRAY, color_noise as f32);
    if tile.blocks {
        tile_color = LIGHT_GRAY;
    }

    char_map.execute(|char_map| {
        draw_char(&char_map, window, pos, tile.glyph, tile_color);
        Ok(())
    });
}

fn draw_entity(entity: &Entity,
               pos: Vector,
               window: &mut Window,
               char_map: &mut Asset<HashMap<u32, Image>>) {
    let _color = 
        match entity.typ {
            EntityType::Monster(monster) => {
                if monster.status == Some(Status::Berserk) {
                    RED
                } else {
                    entity.color
                }
            }

            EntityType::Player(player) => {
                if player.status == Some(Status::Berserk) {
                    RED
                } else {
                    entity.color
                }
            }

            _ => {
                entity.color
            }
        };

    char_map.execute(|char_map| {
        draw_char(&char_map, window, pos, entity.glyph, entity.color);
        return Ok(());
    }).unwrap();
}

// draw functions
fn draw_char(char_map: &HashMap<u32, Image>, window: &mut Window, pos: Vector, chr: char, color: Color) {
    let char_ix = chr as u32;
    let rect = Rectangle::new(pos, Vector::new(16, 16));
    window.draw_ex(&rect,
                   Blended(&char_map[&char_ix], color),
                   Transform::scale(Vector::new(SCALE, SCALE)),
                   SCALE);
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
        draw_rate: 100.0,
        update_rate: MILLIS_PER_UPDATE,
        ..Default::default()
    };
    run::<Game>("Stone Fall", Vector::new(WINDOW_WIDTH, WINDOW_HEIGHT), settings);
}
