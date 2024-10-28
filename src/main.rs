use eframe::{egui, App as EframeApp};
use bevy::{core_pipeline::{core_3d::Transmissive3d, CorePipelinePlugin}, prelude::*, render::{render_phase::DrawFunctions, render_resource::{Extent3d, TextureDescriptor, TextureFormat, TextureUsages}, settings::{Backends, RenderCreation, WgpuSettings}, view::ViewTarget, Extract, RenderApp, RenderPlugin}};

struct BevyApp {
    texture: Option<egui::TextureHandle>,
    bevy_app: App,
    render_target: Handle<Image>,
}

#[derive(Resource, Clone)]
struct RenderTargetResource(Handle<Image>);

#[derive(Resource)]
struct ExtractedTextureData {
    data: Vec<u8>,
    size: Extent3d,
}

impl BevyApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut bevy_app = App::new();
        println!("BevyApp created");

        // Create a channel to send messages from the Bevy app to the render app
        let (app_to_render_sender, _app_to_render_receiver) = async_channel::unbounded();
        let (_render_to_app_sender, render_to_app_receiver) = async_channel::unbounded();

        let render_creation = RenderCreation::Automatic(WgpuSettings{
            backends: Some(bevy::render::settings::Backends::all()),
            power_preference: bevy::render::settings::PowerPreference::HighPerformance,
            ..default()
        });

        println!("RenderCreation created");

        // Add only essential plugins for rendering
        bevy_app
            .add_plugins((
                MinimalPlugins,
                AssetPlugin::default(),
                WindowPlugin {
                    primary_window: None, // No actual window
                    exit_condition: bevy::window::ExitCondition::OnAllClosed,
                    close_when_requested: true,
                },
                bevy::render::pipelined_rendering::PipelinedRenderingPlugin,
                RenderPlugin {
                    render_creation, //: RenderCreation::Manual((render_device, render_queue, render_adapter_info, render_adapter)),  // We need to provide RenderDevice, RenderQueue, RenderAdapterInfo, RenderAdapter
                    synchronous_pipeline_compilation: true,
                },
                bevy::render::texture::ImagePlugin::default(),
                CorePipelinePlugin::default(),
                bevy::pbr::PbrPlugin::default(),
            ));
        println!("Plugins added");

        // Initialize render app channels
        bevy_app.insert_sub_app(
            RenderApp, 
            SubApp::new(),
        );
        let render_app = bevy_app.sub_app_mut(RenderApp).add_systems(ExtractSchedule, extract_texture);
        render_app.insert_resource(DrawFunctions::<Transmissive3d>::default());
        println!("Sub app created");

        render_app.insert_resource(bevy::render::pipelined_rendering::RenderAppChannels::new(app_to_render_sender, render_to_app_receiver));

        let render_target = {
            bevy_app.world_mut().insert_resource(Assets::<Image>::default());
            let mut images = bevy_app.world_mut().get_resource_mut::<Assets<Image>>().unwrap();
            let size = Extent3d {
                width: 512,
                height: 512,
                depth_or_array_layers: 1,
            };
            let mut texture = Image {
                texture_descriptor: TextureDescriptor {
                    size,
                    label: Some("render target"),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: bevy::render::render_resource::TextureDimension::D2,
                    format: bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC,
                    view_formats: &[], // Change this from &[TextureFormat::Rgba8UnormSrgb] 
                    
                },
                ..default()
            };

            texture.data = vec![112; (size.width * size.height * 4) as usize]; // Just fill with a gray color

            images.add(texture)
        };

        bevy_app.insert_resource(RenderTargetResource(render_target.clone()));
        
        bevy_app.add_systems(Startup, setup);

        println!("Systems added");

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
            if image.data.len() == (size.width * size.height * 4) as usize {
                // Convert the pixel data from Bevy's Image (which is in RGBA format)
                // to an Egui ColorImage (which expects RGB format)
                let pixels = image
                    .data
                    .chunks_exact(4)
                    .flat_map(|rgba| rgba[..3].iter().copied())
                    .collect::<Vec<_>>();

                let color_image = egui::ColorImage::from_rgb(
                    [size.width as usize, size.height as usize],
                    &pixels
                );

                if let Some(texture) = &mut self.texture {
                    texture.set(color_image, egui::TextureOptions::default());
                } else {
                    self.texture = Some(ctx.load_texture(
                        "bevy_texture",
                        color_image,
                        Default::default()
                    ));
                }
                // Check if the resulting pixel data has the correct length (if not, from_rgb will panic)
                // if pixels.len() == (size.width * size.height * 3) as usize {
                //     let color_image = egui::ColorImage::from_rgb([size.width as usize, size.height as usize], &pixels);

                //     if let Some(texture) = &mut self.texture {
                //         // Update the existing texture
                //         texture.set(color_image, egui::TextureOptions::default());
                //     } else {
                //         // Create a new Egui texture
                //         self.texture = Some(ctx.load_texture("bevy_texture", color_image, Default::default()));
                //     }
                // }
            } else {
                println!("Invalid image data length: {}", image.data.len());
            }
        } else {
            println!("Image not found");
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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut render_target: ResMut<RenderTargetResource>,
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
    let render_target_texture = images.add(texture);

    // Spawn a light and the camera
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 3.0)),
        ..default()
    });

    commands.insert_resource(AmbientLight {
        brightness: 0.3,
        ..default()
    });

    // circular base
    commands.spawn(PbrBundle {
        mesh: meshes.add(Circle::new(4.0)),
        material: materials.add(Color::WHITE),
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        material: materials.add(Color::srgb_u8(124, 144, 255)),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });

    let translation = Vec3::new(0., -5.0, 5.);

    // Spawn a camera that renders to the render target
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                target: bevy::render::camera::RenderTarget::Image(render_target_texture.clone()),
                ..default()
            },
            transform: Transform::from_translation(translation),
            ..default()
        },
    ));

    // Check if RenderTargetResource is already inserted, if not insert it, otherwise update it
    *render_target = RenderTargetResource(render_target_texture); // Modify the resource directly

    println!("Setup done");
}

// System to extract the rendered image:
#[derive(Bundle)]
struct CameraBundle {
    camera: Camera,
    view_target: ViewTarget,
}

fn extract_texture(
    mut commands: Commands,
    images: Extract<Res<Assets<Image>>>,
    render_target: Extract<Res<RenderTargetResource>>,
) {
    println!("Extracting texture");
    if let Some(image) = images.get(&render_target.0) {
        // Clone the texture data
        let texture_data = image.data.clone();

        // Create an Extent3d representing the texture size
        let size = Extent3d {
            width: image.texture_descriptor.size.width,
            height: image.texture_descriptor.size.height,
            depth_or_array_layers: 1,
        };

        // Insert the cloned texture data as a resource or send it to other systems
        commands.insert_resource(ExtractedTextureData{data: texture_data, size});
    }
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