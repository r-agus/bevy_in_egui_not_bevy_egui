use eframe::{egui, App as EframeApp};
use bevy::{app::PanicHandlerPlugin, core_pipeline::CorePipelinePlugin, pbr::GpuMeshPreprocessPlugin, prelude::*, render::{pipelined_rendering::PipelinedRenderingPlugin, render_resource::{Extent3d, TextureDescriptor, TextureFormat, TextureUsages}, RenderPlugin}};

struct BevyApp {
    texture: Option<egui::TextureHandle>,
    bevy_app: App,
    render_target: Handle<Image>,
}

#[derive(Resource)]
struct RenderTargetResource(Handle<Image>);

impl BevyApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut bevy_app = App::new();
        println!("BevyApp created");
        // bevy_app.add_plugins(MinimalPlugins);       // Do not load DefaultPlugins!!! It will cause a panic
        // bevy_app.add_plugins(bevy::log::LogPlugin::default());
        // bevy_app.add_plugins(bevy::asset::AssetPlugin::default());
        // bevy_app.add_plugins(bevy::render::RenderPlugin::default());
        // println!("RenderPlugin added");
        // bevy_app.add_plugins(bevy::render::texture::ImagePlugin::default());
        // // bevy_app.add_plugins(bevy::core_pipeline::CorePipelinePlugin);
        // bevy_app.add_plugins(bevy::core_pipeline::core_3d::Core3dPlugin);
        // bevy_app.add_plugins(bevy::core_pipeline::core_2d::Core2dPlugin);  // This is what makes DefaultPlugins panic!
        // println!("CorePipelinePlugin added");
        // bevy_app.add_plugins(bevy::pbr::PbrPlugin::default());
        // bevy_app.add_plugins(bevy::ui::UiPlugin::default());
        // bevy_app.add_plugins(bevy::input::InputPlugin::default());
        // bevy_app.add_plugins(bevy::sprite::SpritePlugin::default());
        // bevy_app.add_plugins(bevy::text::TextPlugin::default());
        // bevy_app.add_plugins(bevy::scene::ScenePlugin::default());
        // bevy_app.add_plugins(bevy::a11y::AccessibilityPlugin);
        // bevy_app.add_plugins(HierarchyPlugin);
        // bevy_app.add_plugins(bevy::diagnostic::DiagnosticsPlugin);
        // bevy_app.add_plugins(PipelinedRenderingPlugin::default());
        // bevy_app.add_plugins(bevy::window::WindowPlugin::default());
        bevy_app.add_plugins(CorePipelinePlugin);
        println!("CorePipelinePlugin added");
        bevy_app.add_plugins(DefaultPlugins);
        println!("DefaultPlugins added");
        bevy_app.add_systems(Startup, setup);
        println!("Systems added");

        let render_target = {
            bevy_app.world_mut().insert_resource(Assets::<Image>::default());
            let mut images = bevy_app.world_mut().get_resource_mut::<Assets<Image>>().unwrap();
            let size = Extent3d {
                width: 512,
                height: 512,
                depth_or_array_layers: 1,
            };
            let texture = Image {
                texture_descriptor: TextureDescriptor {
                    size,
                    label: None,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: bevy::render::render_resource::TextureDimension::D2,
                    format: bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    view_formats: &[TextureFormat::Rgba8UnormSrgb],
                    
                },
                ..default()
            };
            images.add(texture)
        };
        Self { 
            texture: None,
            bevy_app,
            render_target,
        }
    }
}

impl EframeApp for BevyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) { 
        self.bevy_app.update();
        let images = self.bevy_app.world_mut().get_resource_mut::<Assets<Image>>().unwrap();

        if let Some(image) = images.get(&self.render_target) {
            let size = image.texture_descriptor.size;

            // Ensure the pixel data is available and has the correct length
            if !image.data.is_empty() && image.data.len() == (size.width * size.height * 4) as usize {
                // Convert the pixel data from Bevy's Image (which is in RGBA format)
                // to an Egui ColorImage (which expects RGB format)
                let pixels = image
                    .data
                    .chunks_exact(4)
                    .flat_map(|rgba| rgba[..3].iter().copied())
                    .collect::<Vec<_>>();

                // Check if the resulting pixel data has the correct length
                if pixels.len() == (size.width * size.height * 3) as usize {
                    let color_image = egui::ColorImage::from_rgb([size.width as usize, size.height as usize], &pixels);

                    if let Some(texture) = &mut self.texture {
                        // Update the existing texture
                        texture.set(color_image, egui::TextureOptions::default());
                    } else {
                        // Create a new Egui texture
                        self.texture = Some(ctx.load_texture("bevy_texture", color_image, Default::default()));
                    }
                }
            } else {
                println!("Invalid image data length: {}", image.data.len());

            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(texture) = &self.texture {
                ui.image(texture);
            }
            ui.label("Bevy render target");
        });
    }
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    // Create a render target texture
    let size = Extent3d {
        width: 512,
        height: 512,
        depth_or_array_layers: 1,
    };
    let mut texture = Image {
        texture_descriptor: TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: bevy::render::render_resource::TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            label: None,
            view_formats: &[TextureFormat::Rgba8UnormSrgb],
        },
        ..default()
    };
    texture.resize(size);
    let render_target = images.add(texture);

    // Spawn a light and the camera
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 3.0)),
        ..default()
    });

    commands.insert_resource(AmbientLight {
        brightness: 0.3,
        ..default()
    });

    let translation = Vec3::new(0., -5.0, 5.);

    // Spawn a camera that renders to the render target
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                target: bevy::render::camera::RenderTarget::Image(render_target.clone()),
                ..default()
            },
            transform: Transform::from_translation(translation),
            ..default()
        },
    ));

    // Store the render target handle for future use
    commands.insert_resource(RenderTargetResource(render_target));
}

fn main() {
    let options = eframe::NativeOptions {
        ..Default::default()
    };

    eframe::run_native(
        "Bevy in egui",
        options,
        Box::new(|cc| Ok(Box::new(BevyApp::new(cc)))),
    ).unwrap();
}