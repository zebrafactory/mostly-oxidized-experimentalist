use avian3d::prelude::*;
use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};
use std::f32::consts::FRAC_PI_2;

const FLOOR_RADIUS: f32 = 100.0;
const FLOOR_HALF_HEIGHT: f32 = 0.5;
const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;
const PLAYER_RADIUS: f32 = 0.5;

const FLOOR_COLOR: Color = Color::srgb(0.9, 0.7, 0.8);
const BEAM_COLOR: Color = Color::srgb(0.4, 0.7, 0.9);
const BALL_COLOR: Color = Color::srgb(0.8, 0.8, 0.8);

// FIXME: depending on the exact value here, the balls can fall and be still or fall and move away
// from the camera. This might be a bug in Avian3d as it seems the size should not tilt the
// behavior like that.
const BALL_RADIUS: f32 = 0.6;

#[derive(Component)]
struct Resetable {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

impl Resetable {
    fn from_xyz(x: f32, y: f32, z: f32) -> (Self, Transform) {
        let transform = Transform::from_xyz(x, y, z);
        Self::from_transform(transform)
    }

    fn from_transform(transform: Transform) -> (Self, Transform) {
        (
            Resetable {
                translation: transform.translation,
                rotation: transform.rotation,
                scale: transform.scale,
            },
            transform,
        )
    }

    fn reset(&self, transform: &mut Transform) {
        transform.translation = self.translation;
        transform.rotation = self.rotation;
        transform.scale = self.scale;
    }
}

#[derive(Component)]
struct Ball;

#[derive(Event)]
struct ResetWorld;

#[derive(Event)]
struct ModifyDensity;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, (mouse_move, button_input).chain())
        .add_observer(ray_cast_system)
        .add_observer(reset_world)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // The Player (a camera in a collider)
    commands.spawn((
        RigidBody::Dynamic,
        Collider::sphere(PLAYER_RADIUS),
        Camera3d::default(),
        Resetable::from_xyz(0.0, 3.0, 0.0),
    ));

    // The "floor"
    commands.spawn((
        RigidBody::Static,
        Collider::cylinder(FLOOR_RADIUS, FLOOR_HALF_HEIGHT),
        Mesh3d(meshes.add(Cylinder::new(FLOOR_RADIUS, FLOOR_HALF_HEIGHT))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: FLOOR_COLOR,
            ..default()
        })),
        Transform::from_xyz(0.0, -FLOOR_HALF_HEIGHT * 2.0, 0.0),
    ));

    commands.spawn((
        RigidBody::Static,
        Collider::sphere(BALL_RADIUS),
        Mesh3d(meshes.add(Sphere::new(BALL_RADIUS))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: BALL_COLOR,
            ..default()
        })),
        Resetable::from_xyz(0.0, 0.0, -4.0),
    ));

    // The balance beam
    commands.spawn((
        RigidBody::Dynamic,
        Collider::cuboid(5.0, 0.25, 0.5),
        Mesh3d(meshes.add(Cuboid::new(5.0, 0.25, 0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: BEAM_COLOR,
            ..default()
        })),
        Resetable::from_xyz(0.0, 2.0, -4.0),
    ));

    for x_offset in [-2.0, 2.0] {
        commands.spawn((
            Ball,
            RigidBody::Dynamic,
            Collider::sphere(BALL_RADIUS),
            Mesh3d(meshes.add(Sphere::new(BALL_RADIUS))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: BALL_COLOR,
                ..default()
            })),
            Resetable::from_xyz(x_offset, 2.5, -4.0),
        ));
    }

    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            contact_shadows_enabled: true,
            intensity: 777_000.0,
            ..default()
        },
        Transform::from_xyz(5.0, 7.0, 5.0),
    ));
}

fn button_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut cursor_options: Single<&mut CursorOptions>,
    mut commands: Commands,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        if !cursor_options.visible {
            cursor_options.visible = true;
            cursor_options.grab_mode = CursorGrabMode::None;
        }
    }
    if mouse.any_just_pressed([MouseButton::Left, MouseButton::Right]) {
        if cursor_options.visible {
            cursor_options.visible = false;
            cursor_options.grab_mode = CursorGrabMode::Locked;
        } else {
            commands.trigger(ModifyDensity);
        }
    }
    if keyboard.just_pressed(KeyCode::KeyR) {
        commands.trigger(ResetWorld);
    }
}

fn mouse_move(
    motion: Res<AccumulatedMouseMotion>,
    cursor_options: Single<&mut CursorOptions>,
    mut transform: Single<&mut Transform, With<Camera3d>>,
) {
    if !cursor_options.visible {
        let delta = motion.delta;
        if delta.x != 0.0 || delta.y != 0.0 {
            let delta_yaw = -delta.x * 0.003;
            let delta_pitch = -delta.y * 0.002;
            let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
            let yaw = yaw + delta_yaw;
            let pitch = (pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);
            transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
        }
    }
}

fn ray_cast_system(
    _on: On<ModifyDensity>,
    mut ray_cast: MeshRayCast,
    transform: Single<&Transform, With<Camera3d>>,
    query: Query<&MeshMaterial3d<StandardMaterial>, With<Ball>>,
    mut commands: Commands,
) {
    let ray = Ray3d::new(transform.translation, transform.forward());
    let filter = |entity| query.contains(entity);
    let early_exit_test = |_entity| false;
    let settings = MeshRayCastSettings::default()
        .with_filter(&filter)
        .with_early_exit_test(&early_exit_test);
    for (entity, _hit) in ray_cast.cast_ray(ray, &settings) {
        println!("hit on {entity:?}");
        commands.entity(*entity).despawn();
    }
}

fn reset_world(
    _on: On<ResetWorld>,
    mut query: Query<(
        &Resetable,
        &mut Transform,
        &mut LinearVelocity,
        &mut AngularVelocity,
    )>,
) {
    println!("reset world");
    for (resetable, mut transform, mut linear_velocity, mut angular_velocity) in &mut query {
        resetable.reset(&mut transform);
        linear_velocity.0 = Vec3::ZERO;
        angular_velocity.0 = Vec3::ZERO;
    }
}
