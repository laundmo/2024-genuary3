use std::f32::consts::PI;
use std::ops::Add;
use std::ops::Mul;

use bevy::core_pipeline::clear_color::ClearColorConfig;
use bevy::input::mouse::MouseButtonInput;
use bevy::math::cubic_splines::CubicBSpline;
use bevy::math::cubic_splines::CubicCurve;
use bevy::prelude::*;

use bevy::render::camera::RenderTarget;
use bevy::render::render_resource::*;
use bevy::render::view::RenderLayers;
use bevy::sprite::collide_aabb::collide;
use bevy::sprite::MaterialMesh2dBundle;
use bevy::transform::commands;
use bevy::window::PrimaryWindow;
use bevy_screen_diagnostics::ScreenDiagnosticsPlugin;
use bevy_screen_diagnostics::ScreenEntityDiagnosticsPlugin;
use bevy_screen_diagnostics::ScreenFrameDiagnosticsPlugin;
use rand::distributions::Uniform;
use rand::prelude::*;
fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgba(0., 0., 0., 0.1)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "genuary3".to_string(),
                ..default()
            }),
            ..default()
        }))
        // .add_plugins(ScreenDiagnosticsPlugin::default())
        // .add_plugins(ScreenFrameDiagnosticsPlugin)
        // .add_plugins(ScreenEntityDiagnosticsPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                reset_oob,
                draw,
                die,
                next_random_location,
                move_and_paint.after(next_random_location),
            ),
        )
        .run();
}

#[derive(Debug, Component)]
struct MainCamera;

#[derive(Debug, Resource)]
struct SceneImage(Handle<Image>);

fn setup(
    mut commands: Commands,
    q_window: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
) {
    let window = q_window.single();

    let size = Extent3d {
        width: window.resolution.width() as u32,
        height: window.resolution.height() as u32,
        ..default()
    };

    // This is the texture that will be rendered to.
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    // fill image.data with zeroes
    image.resize(size);

    let image_handle = images.add(image);
    commands.insert_resource(SceneImage(image_handle.clone()));

    commands.spawn((
        Camera2dBundle {
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            camera: Camera {
                order: -1,
                target: RenderTarget::Image(image_handle.clone()),
                ..default()
            },
            ..default()
        },
        UiCameraConfig { show_ui: false },
        RenderLayers::layer(1),
    ));
    commands.spawn((Camera2dBundle::default(), MainCamera));

    let scale = Vec3::new(0.5, 0.5, 0.1);
    let pos = Vec3::new(0., 0., 0.);
    let color = Color::rgb(0.25, 0.25, 0.75);
    let mut spawn_droste = |scale, color, pos, rotation: f32| {
        let rotation = Quat::from_rotation_z(rotation.to_radians());
        let pos = pos * scale;
        commands.spawn((
            SpriteBundle {
                transform: Transform {
                    translation: pos,
                    rotation,
                    scale: scale * 0.95,
                },
                texture: image_handle.clone(),

                ..default()
            },
            RenderLayers::layer(0).with(1),
        ));
    };
    let w = size.width as f32;
    let h = size.height as f32;
    spawn_droste(scale, color, pos, 0.);
    spawn_droste(scale, color, Vec3::new(0., -h, 0.), 180.);
    spawn_droste(scale, color, Vec3::new(0., h, 0.), 180.);
    spawn_droste(scale, color, Vec3::new(-(w + h) / 2., -w / 2., 0.), 90.);
    spawn_droste(scale, color, Vec3::new(-(w + h) / 2., w / 2., 0.), 90.);
    spawn_droste(scale, color, Vec3::new((w + h) / 2., -w / 2., 0.), -90.);
    spawn_droste(scale, color, Vec3::new((w + h) / 2., w / 2., 0.), -90.);

    commands.spawn((
        SpatialBundle::default(),
        RandomMove {
            timer: Timer::from_seconds(1.5, TimerMode::Repeating),
            target: CubicBezier::new([[Vec2::ZERO, Vec2::ZERO, Vec2::ZERO, Vec2::ZERO]]).to_curve(),
        },
    ));
}

#[derive(Debug, Component)]
struct Lifetime(Timer);

fn draw(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mb: Res<Input<MouseButton>>,
) {
    if !mb.pressed(MouseButton::Left) {
        return;
    }
    let (camera, camera_transform) = q_camera.single();

    // There is only one primary window, so we can similarly get it from the query:
    let window = q_window.single();

    // check if the cursor is inside the window and get its position
    // then, ask bevy to convert into world coordinates, and truncate to discard Z
    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        // Circle
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: meshes.add(shape::Circle::new(10.).into()).into(),
                material: materials.add(ColorMaterial::from(Color::PURPLE)),
                transform: Transform::from_translation(world_position.extend(0.1)),
                ..default()
            },
            Lifetime(Timer::from_seconds(1., TimerMode::Once)),
            RenderLayers::layer(0).with(1),
        ));
    }
}

fn die(mut q: Query<(Entity, &mut Lifetime)>, time: Res<Time>, mut commands: Commands) {
    for (e, mut l) in &mut q {
        if l.0.tick(time.delta()).just_finished() {
            commands.entity(e).despawn_recursive();
        }
    }
}

fn reset_oob(
    mut q: Query<(Entity, &mut Transform), With<Sprite>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    let (camera, camera_transform) = q_camera.single();
    let window = q_window.single();
    let w = window.resolution.width();
    let h = window.resolution.height();
    let bottom_right = camera.viewport_to_world_2d(camera_transform, Vec2::new(w, h));
    let top_left = camera.viewport_to_world_2d(camera_transform, Vec2::new(0., 0.));

    if let (Some(bottom_right), Some(top_left)) = (bottom_right, top_left) {
        let diff = bottom_right.abs().add(top_left.abs());
        for (e, mut t) in &mut q {
            if let Some(_) = collide(
                t.translation,
                t.scale.truncate(),
                camera_transform.translation(),
                diff,
            ) {
            } else {
                t.translation = Vec3::ZERO;
            }
        }
    }
}

#[derive(Component, Default)]
struct RandomMove {
    timer: Timer,
    target: CubicCurve<Vec2>,
}

fn next_random_location(
    mut q: Query<&mut RandomMove>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    time: Res<Time>,
) {
    let mut rng = rand::thread_rng();
    let (camera, camera_transform) = q_camera.single();
    let window = q_window.single();

    let width = Uniform::from(0.0..window.width());
    let height = Uniform::from(0.0..window.height());
    let offset = Uniform::from(-150.0..150.0);
    for mut rm in &mut q {
        if rm.timer.tick(time.delta()).just_finished() {
            let mut gen_point = || {
                camera
                    .viewport_to_world_2d(
                        camera_transform,
                        Vec2::new(width.sample(&mut rng), height.sample(&mut rng)),
                    )
                    .expect("should always be inside")
            };
            let end = gen_point();
            let start = rm.target.position(1.0);
            let continued = rm.target.position(1.3);
            rm.target = CubicBezier::new([[
                start,
                continued,
                end + Vec2::new(offset.sample(&mut rng), offset.sample(&mut rng)),
                end,
            ]])
            .to_curve();
        }
    }
}

fn move_and_paint(
    mut q_cursor: Query<(&RandomMove, &mut Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
) {
    for (rm, mut t) in &mut q_cursor {
        let fract = rm.timer.percent();
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: meshes.add(shape::Circle::new(10.).into()).into(),
                material: materials.add(ColorMaterial::from(Color::hsl(
                    180. + fract.mul(PI).sin() * 80.,
                    0.5,
                    0.6,
                ))),
                transform: Transform::from_translation(rm.target.position(fract).extend(fract)),
                ..default()
            },
            Lifetime(Timer::from_seconds(1., TimerMode::Once)),
            RenderLayers::layer(0).with(1),
        ));
    }
}
