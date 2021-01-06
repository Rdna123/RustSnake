//#![windows_subsystem = "windows"]

use bevy::input::system::exit_on_esc_system;
use bevy::prelude::*;
use bevy::render::pass::ClearColor;
use rand::prelude::random;
use std::time::Duration;

fn main() {
    App::build()
        .add_resource(WindowDescriptor {
            title: "Snake!".to_string(),
            resizable: false,
            width: 600.0,
            height: 600.0,
           ..Default::default()
        })
        .add_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
        .add_resource(SnakeMoveTimer(Timer::new(
            Duration::from_millis(150. as u64),
            true,
        )))
        .add_resource(SnakeSegments::default())
        .add_resource(LastTailPosition::default())
        .add_resource(Points::default())
        .add_startup_system(setup.system())
        .add_startup_stage("game_setup", SystemStage::single(spawn_snake.system()))
        .add_event::<GrowthEvent>()
        .add_event::<GameOverEvent>()
        .add_event::<ScoreEvent>()
        .add_system(snake_movement.system())
        .add_system(food_spawner.system())
        .add_system(position_translation.system())
        .add_system(snake_growth.system())
        .add_system(snake_eating.system())
        .add_system(size_scaling.system())
        .add_system(exit_on_esc_system.system())
        .add_system(snake_timer.system())
        .add_system(food_overlap_check.system())
        .add_system(point_score.system())
        .add_system(game_over.system())
        .add_plugins(DefaultPlugins)

        .run();
}

//SetUp
struct Materials {
    head_material: Handle<ColorMaterial>,
    segment_material: Handle<ColorMaterial>,
    food_material: Handle<ColorMaterial>,
}

fn setup(commands: &mut Commands,
         mut materials: ResMut<Assets<ColorMaterial>>,
){
    commands.spawn(Camera2dBundle::default());
    commands.insert_resource(Materials {
        head_material: materials.add(Color::rgb(0.7, 0.7, 0.7).into()),
        segment_material: materials.add(Color::rgb(0.3, 0.3, 0.3).into()),
        food_material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
    });
}

//Movement Logic
#[derive(PartialEq, Copy, Clone)]
enum Direction {
    Left,
    Up,
    Right,
    Down,
}

impl Direction {
    fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
    fn scream(self){
        println!("Rrrreeeeh")
    }
}

struct SnakeMoveTimer(Timer);

fn snake_timer(time: Res<Time>, mut snake_timer: ResMut<SnakeMoveTimer>) {
    snake_timer.0.tick(time.delta_seconds());
}

fn snake_movement(
    keyboard_input: Res<Input<KeyCode>>,
    segments: ResMut<SnakeSegments>,
    snake_timer: ResMut<SnakeMoveTimer>,

    mut last_tail_position: ResMut<LastTailPosition>,
    mut game_over_events: ResMut<Events<GameOverEvent>>,
    mut heads: Query<(Entity, &mut SnakeHead)>,
    mut positions: Query<&mut Position>,
) {
    let segment_positions = segments
        .0
        .iter()
        .map(|e| *positions.get_mut(*e).unwrap())
        .collect::<Vec<Position>>();


    last_tail_position.0 = Some(*segment_positions.last().unwrap());

        if let Some((head_entity, mut head)) = heads.iter_mut().next() {
            let mut head_pos = positions.get_mut(head_entity).unwrap();

            let dir: Direction = if keyboard_input.just_pressed(KeyCode::Left) {
                Direction::Left
            } else if keyboard_input.just_pressed(KeyCode::Down) {
                Direction::Down
            } else if keyboard_input.just_pressed(KeyCode::Up) {
                Direction::Up
            } else if keyboard_input.just_pressed(KeyCode::Right) {
                Direction::Right
            } else {
                head.direction
            };



            if dir.opposite() != head.direction{
                head.direction = dir;

            } else { dir.scream(); }

            if !snake_timer.0.finished() {
                return;
            }
            match &head.direction {
                Direction::Left => {
                    head_pos.x -= 1;
                }
                Direction::Right => {
                    head_pos.x += 1;
                }
                Direction::Up => {
                    head_pos.y += 1;
                }
                Direction::Down => {
                    head_pos.y -= 1;
                }
            }
            if head_pos.x < 0
                || head_pos.y < 0
                || head_pos.x as u32 >= ARENA_WIDTH
                || head_pos.y as u32 >= ARENA_HEIGHT
            {
                game_over_events.send(GameOverEvent);
            }
            if segment_positions.contains(&head_pos) {
                game_over_events.send(GameOverEvent);
            }
        }
    segment_positions.iter().zip(segments.0.iter().skip(1)).for_each(|(pos, segment)| {
        *positions.get_mut(*segment).unwrap() = *pos;
    });
}

//Snake

struct SnakeHead {
    direction: Direction,
}





fn spawn_snake(
    commands: &mut Commands,
    materials: Res<Materials>,
    mut segments: ResMut<SnakeSegments>,
) {
    segments.0 = vec![
        commands
            .spawn(SpriteBundle {
                material: materials.head_material.clone(),
                ..Default::default()
            })
            .with(SnakeHead { direction: Direction::Up })
            .with(SnakeSegment)
            .with(Position { x: 3, y: 3 })
            .with(Size::square(0.8))
            .current_entity()
            .unwrap(),
        spawn_segment(commands, &materials.segment_material, Position { x: 3, y: 2 }),
    ];
}

struct SnakeSegment;

#[derive(Default)]
struct SnakeSegments(Vec<Entity>);

fn spawn_segment(
    commands: &mut Commands,
    material: &Handle<ColorMaterial>,
    position: Position,
) -> Entity {
    commands
        .spawn(SpriteBundle { material: material.clone(), ..Default::default() })
        .with(SnakeSegment)
        .with(position)
        .with(Size::square(0.65))
        .current_entity()
        .unwrap()
}

//Eating and Growth
fn snake_eating(
    commands: &mut Commands,
    snake_timer: ResMut<SnakeMoveTimer>,
    mut growth_events: ResMut<Events<GrowthEvent>>,
    mut score_event: ResMut<Events<ScoreEvent>>,
    food_positions: Query<(Entity, &Position), With<Food>>,
    head_positions: Query<&Position, With<SnakeHead>>,
) {
    if !snake_timer.0.finished() {
        return;
    }
    for head_pos in head_positions.iter() {
        for (ent, food_pos) in food_positions.iter() {
            if food_pos == head_pos {
                commands.despawn(ent);
                growth_events.send(GrowthEvent);
                score_event.send(ScoreEvent);
            }
        }
    }
}
struct GrowthEvent;

#[derive(Default)]
struct LastTailPosition(Option<Position>);

fn snake_growth(
    commands: &mut Commands,
    last_tail_position: Res<LastTailPosition>,
    growth_events: Res<Events<GrowthEvent>>,
    mut segments: ResMut<SnakeSegments>,
    mut growth_reader: Local<EventReader<GrowthEvent>>,
    materials: Res<Materials>,
) {
    if growth_reader.iter(&growth_events).next().is_some() {
        segments.0.push(spawn_segment(
            commands,
            &materials.segment_material,
            last_tail_position.0.unwrap(),
        ));
    }
}

//Food
struct Food;

struct FoodSpawnTimer(Timer);
impl Default for FoodSpawnTimer {
    fn default() -> Self {
        Self(Timer::new(Duration::from_millis(1000), true))
    }
}

fn food_spawner(
    commands: &mut Commands,
    materials: Res<Materials>,
    time: Res<Time>,
    mut timer: Local<FoodSpawnTimer>,
) {
    if timer.0.tick(time.delta_seconds()).finished() {
        commands
            .spawn(SpriteBundle {
                material: materials.food_material.clone(),
                ..Default::default()
            })
            .with(Food)
            .with(Position {
                x: (random::<f32>() * ARENA_WIDTH as f32) as i32,
                y: (random::<f32>() * ARENA_HEIGHT as f32) as i32,
            })
            .with(Size::square(0.8));
    }
}

fn food_overlap_check(
    commands: &mut Commands,
    segment_position: Query<&Position, With<SnakeSegment>>,
    food_position: Query<(Entity, &Position), With<Food>>,
    head_position: Query<&Position, With<SnakeHead>>,
) {
    for segment_pos in segment_position.iter() {
        for head_pos in head_position.iter() {
            for (ent, food_pos) in food_position.iter() {
                if food_pos == segment_pos {
                    commands.despawn(ent);
                }
                if food_pos == head_pos {
                    commands.despawn(ent);
                }
            }
        }
    }
}

//size things

const ARENA_WIDTH: u32 = 10;
const ARENA_HEIGHT: u32 = 10;

#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
struct Position {
    x: i32,
    y: i32,
}

struct Size {
    width: f32,
    height: f32,
}
impl Size {
    pub fn square(x: f32) -> Self {
        Self { width: x, height: x }
    }
}

fn size_scaling(windows: Res<Windows>, mut q: Query<(&Size, &mut Sprite)>) {
    let window = windows.get_primary().unwrap();
    for (sprite_size, mut sprite) in q.iter_mut() {
        sprite.size = Vec2::new(
            sprite_size.width / ARENA_WIDTH as f32 * window.width() as f32,
            sprite_size.height / ARENA_HEIGHT as f32 * window.height() as f32,
        );
    }
}

fn position_translation(
    windows: Res<Windows>,
    mut q: Query<(&Position, &mut Transform)>,
) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }
    let window = windows.get_primary().unwrap();
    for (pos, mut transform) in q.iter_mut() {
        transform.translation = Vec3::new(
            convert(pos.x as f32, window.width() as f32, ARENA_WIDTH as f32),
            convert(pos.y as f32, window.height() as f32, ARENA_HEIGHT as f32),
            0.0,
        );
    }
}

//GameOver
struct GameOverEvent;

fn game_over(
    commands: &mut Commands,
    mut reader: Local<EventReader<GameOverEvent>>,
    game_over_events: Res<Events<GameOverEvent>>,
    materials: Res<Materials>,
    mut total : ResMut<Points>,
    segments_res: ResMut<SnakeSegments>,
    food: Query<Entity, With<Food>>,
    segments: Query<Entity, With<SnakeSegment>>,
) {
    if reader.iter(&game_over_events).next().is_some() {
        for ent in food.iter().chain(segments.iter()) {
            commands.despawn(ent);
        }
        if total.score != 0 {
            println!("Points for this round are {}", total.score);
        }

        total.score = 0;
        spawn_snake(commands, materials, segments_res);
    }
}

struct ScoreEvent;
struct Points{
    score: i32,
}

impl Default for Points{
    fn default() -> Self {
       Self{
           score: 0
       }
    }
}

fn point_score(
   // commands: &mut Commands,
    mut total : ResMut<Points>,
    mut reader: Local<EventReader<ScoreEvent>>,
    mut reader_game_over: Local<EventReader<GameOverEvent>>,
    score_event: Res<Events<ScoreEvent>>,
    game_over_event: Res<Events<GameOverEvent>>,
){

    if reader.iter(&score_event).next().is_some(){
        total.score +=1;
    }
    if reader_game_over.iter(&game_over_event).next().is_some(){

    }
}
