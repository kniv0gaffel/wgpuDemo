use crate::core::texture;
use crate::core::model::{DrawModel,  DrawLight};
use crate::app::world;
use crate::core::camera;


use winit::window::Window;
use winit:: event::*;
use winit::window::CursorGrabMode;










pub struct Renderer<'a>
{
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,// The window must be declared after, the surface contains unsafe references to the window's resources.
    window: Window,
    camera: camera::Camera,
    render_pipelines : Vec<wgpu::RenderPipeline>,
    frame_buffers : Vec<FrameBuffer>,
    passes : Vec<wgpu::RenderPass<'a>>,
    world : world::World,
    mouse_locked: bool
}


impl Renderer<'static>
{
    pub async fn new(window: Window) -> Self
    {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(
            wgpu::InstanceDescriptor 
            {
                backends: wgpu::Backends::all(),
                dx12_shader_compiler: Default::default(),
                ..Default::default()
            }
        );
        // # Safety
        // The surface needs to live as long as the window that created it.
        // State owns the window so this should be safe.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions 
            {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor 
            {
                features: wgpu::Features::empty(),
                limits: 
                    if cfg!(target_arch = "wasm32") { wgpu::Limits::downlevel_webgl2_defaults() }
                    else { wgpu::Limits::default() },
                label: None,
            },
            None, // Trace path
        ).await.unwrap();


        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .copied()
            .find(|f| f.is_srgb())            
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration 
        {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);
        

        let _modes = &surface_caps.present_modes;
        
        let world = world::World::new(&config, &device, &queue);
        

        // make pipelines 
        let pixel_pipeline : wgpu::RenderPipeline;
        let final_pipeline : wgpu::RenderPipeline;
        {
            let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/shader.wgsl"));
            let floorshader = device.create_shader_module(wgpu::include_wgsl!("shaders/floor.wgsl"));
            let finalshader = device.create_shader_module(wgpu::include_wgsl!("shaders/final.wgsl"));
        }



        let low_res_texture = texture::Texture::create_blank_texture(
            &self.device, 
            &self.config.width / 4 , &self.config.height / 4,
            "low_res_texture", 
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC, wgpu::TextureFormat::Rgba8UnormSrgb);
        // Create a bind group layout


        // Create a bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&low_res_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });



        let frame_buffers = vec![
            FrameBuffer
            {
                framebuffer: None, 
                depth_texture: depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture") , 
                size: size
                bind_group: None,
                bind_group_layout: None,
            },
            FrameBuffer
            {
                framebuffer: low_res_texture,
                depth_texture: depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture") ,
                size: size
                bind_group: Some(bind_group),
                bind_group_layout: Some(bind_group_layout),
            },
        ];

        let camera = camera::Camera::new(
            cgmath::Point3::new(0.0, 0.0, 3.0),
            cgmath::Deg(0.0),
            cgmath::Deg(0.0),
            camera::Projection::new(config.width, config.height, cgmath::Deg(45.0), 0.1, 100.0),
            camera::CameraController::new(5.0, 0.4),
            &device,
        );


        Self
        {
            surface,
            device,
            queue,
            config,
            size,
            window,
            camera,
            render_pipelines : vec![final_render_pipeline, render_pipeline, floor_render_pipeline],
            frame_buffers,
            passes : vec![],
            world,
            mouse_locked: false,
        }

    }


    pub fn window(&self) -> &Window
    {
        &self.window
    }

    // impl State
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>)
    {
        if new_size.width > 0 && new_size.height > 0 
        {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
            self.world.camera.projection.resize(new_size.width, new_size.height);
        }
    }



// impl State
    pub fn window_input(&mut self, event: &WindowEvent) -> bool
    {
        match event 
        {
            WindowEvent::KeyboardInput {
                    input: KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Tab),
                        ..
                    },
                    ..
            } => 
            {
                if self.mouse_locked == false
                {
                    self.window.set_cursor_grab(CursorGrabMode::Confined).or_else(|_| 
                        self.window.set_cursor_grab(CursorGrabMode::Locked)).unwrap();
                    self.window.set_cursor_visible(false);
                    self.mouse_locked = true; 
                }
                else
                {
                    self.window.set_cursor_grab(CursorGrabMode::None).unwrap();
                    self.window.set_cursor_visible(true);
                    self.mouse_locked = false;
                }
                true
            }
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(key),
                        state,
                        ..
                    },
                ..
            } => 
            { 
                self.world.camera.camera_controller.process_keyboard(*key, *state);
                self.window.set_title(&format!("{:?}", self.world.camera.camera.position));
                true
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.world.camera.camera_controller.process_scroll(delta);
                self.window.set_title(&format!("{:?}", self.world.camera.camera.position));
                true
            }
            _ => false,
        }
    }



    pub fn device_input(&mut self, event : &DeviceEvent) -> bool
    {
        match event
        {
            DeviceEvent::MouseMotion{ delta, } if self.mouse_locked == true => 
            {
                self.world.camera.camera_controller.process_mouse(delta.0, delta.1);
                true
            }
            _ => false,
        }
    }



    pub fn update(&mut self, dt: instant::Duration)
    {
        self.world.camera.camera_controller.update_camera(&mut self.world.camera.camera, dt);
        self.world.camera.camera_uniform.update_view_proj(&self.world.camera.camera, &self.world.camera.projection);
        self.queue.write_buffer(&self.world.camera.camera_buffer, 0, bytemuck::cast_slice(&[self.world.camera.camera_uniform]));
    }

// impl State

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> 
    {








        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor 
            {
                label: Some("Render Encoder"),
            }
        );
        {
            let mut render_pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor 
                {
                    label: Some("Pixel Pass"),
                    color_attachments: &[Some(
                        wgpu::RenderPassColorAttachment 
                        {
                            view: &low_res_texture_view,
                            resolve_target: None,
                            ops: wgpu::Operations 
                            {
                                load: wgpu::LoadOp::Clear(
                                    wgpu::Color 
                                    {
                                        r: 0.15,
                                        g: 0.15,
                                        b: 0.15,
                                        a: 1.0,
                                    }   
                                    // self.color
                                ),
                                store: wgpu::StoreOp::Store,
                            },
                        }
                    )],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_texture.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0), // 1.
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                }
            );

            render_pass.set_pipeline(&self.render_pipeline[2]);
            render_pass.set_bind_group(0, &self.world.entities[1].material.diffuse_bind_group, &[]); // NEW!
            render_pass.set_bind_group(1, &self.world.camera.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.world.entities[1].mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.world.entities[1].instances.as_ref().unwrap().instance_buffer.slice(..));
            render_pass.set_index_buffer(self.world.entities[1].mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.world.entities[1].mesh.num_indices, 0, 0..1 as _);
            render_pass.set_pipeline(&self.render_pipeline[1]);
            render_pass.set_bind_group(0, &self.world.entities[0].material.diffuse_bind_group, &[]); // NEW!
            render_pass.set_bind_group(1, &self.world.camera.camera_bind_group, &[]);
            use crate::components::model::DrawModel;
            render_pass.draw_mesh_instanced(&self.obj_model.meshes[0], &self.obj_model.materials[0], 0..1, &self.world.camera.camera_bind_group);
        }  
        {
            let mut render_pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor 
                {
                    label: Some("Final Pass"),
                    color_attachments: &[Some(
                        wgpu::RenderPassColorAttachment 
                        {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations 
                            {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        }
                    )],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                }
            );

            render_pass.set_pipeline(&self.render_pipeline[0]);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.screen_quad.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.screen_quad.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..1 as _);
        }

        //
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }


}
