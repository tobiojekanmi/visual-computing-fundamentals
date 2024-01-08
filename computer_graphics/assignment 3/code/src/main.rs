// Uncomment these following global attributes to silence most warnings of "low" interest:
/*
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unreachable_code)]
#![allow(unused_mut)]
#![allow(unused_unsafe)]
#![allow(unused_variables)]
*/
extern crate nalgebra_glm as glm;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::{mem, os::raw::c_void, ptr};

mod mesh;
mod scene_graph;
mod shader;
mod toolbox;
mod util;

use glutin::event::{
    DeviceEvent,
    ElementState::{Pressed, Released},
    Event, KeyboardInput,
    VirtualKeyCode::{self, *},
    WindowEvent,
};
use glutin::event_loop::ControlFlow;
use scene_graph::SceneNode;

// initial window size
const INITIAL_SCREEN_W: u32 = 800;
const INITIAL_SCREEN_H: u32 = 600;

// == // Helper functions to make interacting with OpenGL a little bit prettier. You *WILL* need these! // == //

// Get the size of an arbitrary array of numbers measured in bytes
// Example usage:  pointer_to_array(my_array)
fn byte_size_of_array<T>(val: &[T]) -> isize {
    std::mem::size_of_val(&val[..]) as isize
}

// Get the OpenGL-compatible pointer to an arbitrary array of numbers
// Example usage:  pointer_to_array(my_array)
fn pointer_to_array<T>(val: &[T]) -> *const c_void {
    &val[0] as *const T as *const c_void
}

// Get the size of the given type in bytes
// Example usage:  size_of::<u64>()
fn size_of<T>() -> i32 {
    mem::size_of::<T>() as i32
}

// Get an offset in bytes for n units of type T, represented as a relative pointer
// Example usage:  offset::<u64>(4)
fn offset<T>(n: u32) -> *const c_void {
    (n * mem::size_of::<T>() as u32) as *const T as *const c_void
}

// Get a null pointer (equivalent to an offset of 0)
// ptr::null()

/*********************************************************************/
/* Task 1 - Modified Create VAO function */
/*********************************************************************/
unsafe fn create_vao(
    vertices: &Vec<f32>,
    indices: &Vec<u32>,
    colors: &Vec<f32>,
    normals: &Vec<f32>,
) -> u32 {
    // 1. Generate a VAO and bind it
    let num_vao = 1;
    let mut vao_id: u32 = 0;
    gl::GenVertexArrays(num_vao, &mut vao_id);
    gl::BindVertexArray(vao_id);

    // 2. Configure the Vertex Array Object for Vertex Coordinates
    // 2.1. Generate a VBO and bind it
    let num_coord_vbo = 1;
    let mut coord_vbo_id: u32 = 0;
    gl::GenBuffers(num_coord_vbo, &mut coord_vbo_id);
    gl::BindBuffer(gl::ARRAY_BUFFER, coord_vbo_id);

    // 2.2. Fill it with data
    gl::BufferData(
        gl::ARRAY_BUFFER,
        byte_size_of_array(vertices),
        pointer_to_array(vertices),
        gl::STATIC_DRAW,
    );

    // 2.3. Configure a VAP for the vertex coordinates and enable it
    gl::EnableVertexAttribArray(0);
    gl::VertexAttribPointer(
        0,           // Index of the generic vertex attribute to modify
        3,           // Number of vertex attributes (i.e., 3 coordinate values -> [x, y, z])
        gl::FLOAT,   // Data type of each component in the array
        gl::FALSE,   // OpenGL should not normalize the values in the buffer
        0,           // let OpenGL infer the correct stride attributes
        ptr::null(), // 0 offset
    );

    // 3. Configure the Vertex Array Object for Colors
    // 3.1. Generate a VBO and bind it
    let num_color_vbo = 1;
    let mut color_vbo_id: u32 = 0;
    gl::GenBuffers(num_color_vbo, &mut color_vbo_id);
    gl::BindBuffer(gl::ARRAY_BUFFER, color_vbo_id);

    // 3.2. Fill the color VBO with data
    gl::BufferData(
        gl::ARRAY_BUFFER,
        byte_size_of_array(colors),
        pointer_to_array(colors),
        gl::STATIC_DRAW,
    );

    // 3.3. Configure a VAP for the vertex colors and enable it
    gl::EnableVertexAttribArray(1);
    gl::VertexAttribPointer(
        1,           // Index of the generic vertex attribute to modify
        4,           // Number of vertex attributes (i.e., 4 color floats -> [R, G, B, A])
        gl::FLOAT,   // Data type of each component in the array
        gl::FALSE,   // OpenGL should not normalize the values in the buffer
        0,           // let OpenGL infer the correct stride attributes
        ptr::null(), // 0 offset
    );

    // 4. Configure the Vertex Array Object for Normals
    // 4.1. Generate a VBO and bind it
    let num_normal_vbo = 1;
    let mut normal_vbo_id: u32 = 0;
    gl::GenBuffers(num_normal_vbo, &mut normal_vbo_id);
    gl::BindBuffer(gl::ARRAY_BUFFER, normal_vbo_id);

    // 4.2. Fill the color VBO with data
    gl::BufferData(
        gl::ARRAY_BUFFER,
        byte_size_of_array(normals),
        pointer_to_array(normals),
        gl::STATIC_DRAW,
    );

    // 4.3. Configure a VAP for the vertex colors and enable it
    gl::EnableVertexAttribArray(2);
    gl::VertexAttribPointer(
        2,           // Index of the generic vertex attribute to modify
        3,           // Number of vertex attributes (i.e., 3 normal coordinates -> [X, Y, Z])
        gl::FLOAT,   // Data type of each component in the array
        gl::FALSE,   // OpenGL should not normalize the values in the buffer
        0,           // let OpenGL infer the correct stride attributes
        ptr::null(), // 0 offset
    );

    // * Generate a IBO and bind it
    let num_ibo = 1;
    let mut ibo_id: u32 = 0;
    gl::GenBuffers(num_ibo, &mut ibo_id);
    gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ibo_id);

    // * Fill it with data
    gl::BufferData(
        gl::ELEMENT_ARRAY_BUFFER,
        byte_size_of_array(indices),
        pointer_to_array(indices),
        gl::STATIC_DRAW,
    );

    // * Return the ID of the VAO
    return vao_id;
}

/*********************************************************************/
/* Task 2c and 3 - Recursive draw_scene function */
/*********************************************************************/
unsafe fn draw_scene(
    node: &scene_graph::SceneNode,
    view_projection_matrix: &glm::Mat4,
    transformation_so_far: &glm::Mat4,
) {
    // Perform any logic needed before drawing the node
    let mut model_matrix = glm::identity();

    model_matrix = glm::translate(&model_matrix, &node.position);
    model_matrix = glm::translate(&model_matrix, &node.reference_point);
    model_matrix = glm::scale(&model_matrix, &node.scale);
    model_matrix = glm::rotate_x(&model_matrix, node.rotation.x);
    model_matrix = glm::rotate_y(&model_matrix, node.rotation.y);
    model_matrix = glm::rotate_z(&model_matrix, node.rotation.z);
    model_matrix = glm::translate(&model_matrix, &-node.reference_point);
    model_matrix = transformation_so_far * model_matrix;

    let mvp_matrix: glm::Mat4 = view_projection_matrix * model_matrix;

    // Check if node is drawable, if so: set uniforms, bind VAO and draw VAO
    if node.index_count > 0 {
        gl::BindVertexArray(node.vao_id);

        gl::UniformMatrix4fv(0, 1, gl::FALSE, mvp_matrix.as_ptr());
        gl::UniformMatrix4fv(1, 1, gl::FALSE, model_matrix.as_ptr());

        gl::DrawElements(
            gl::TRIANGLES,
            node.index_count,
            gl::UNSIGNED_INT,
            ptr::null(),
        );
    }

    // Recurse
    for &child in &node.children {
        draw_scene(&*child, view_projection_matrix, &model_matrix);
    }
}

/*********************************************************************/
/* Task 2b, 4 and 6 - Helicopter struct & implementation */
/*********************************************************************/
struct Helicopter {
    helicopter_node: mem::ManuallyDrop<std::pin::Pin<std::boxed::Box<scene_graph::SceneNode>>>,
    body_node: mem::ManuallyDrop<std::pin::Pin<std::boxed::Box<scene_graph::SceneNode>>>,
    door_node: mem::ManuallyDrop<std::pin::Pin<std::boxed::Box<scene_graph::SceneNode>>>,
    main_rotor_node: mem::ManuallyDrop<std::pin::Pin<std::boxed::Box<scene_graph::SceneNode>>>,
    tail_rotor_node: mem::ManuallyDrop<std::pin::Pin<std::boxed::Box<scene_graph::SceneNode>>>,
}

impl Helicopter {
    unsafe fn new(
        helicopter_object_file: &mesh::Helicopter,
        starting_position: &glm::Vec3,
        starting_rotation: &glm::Vec3,
    ) -> Self {
        // Create VAOs for each body part
        let helicopter_body_vao = create_vao(
            &helicopter_object_file.body.vertices,
            &helicopter_object_file.body.indices,
            &helicopter_object_file.body.colors,
            &helicopter_object_file.body.normals,
        );
        let helicopter_door_vao = create_vao(
            &helicopter_object_file.door.vertices,
            &helicopter_object_file.door.indices,
            &helicopter_object_file.door.colors,
            &helicopter_object_file.door.normals,
        );
        let helicopter_main_rotor_vao = create_vao(
            &helicopter_object_file.main_rotor.vertices,
            &helicopter_object_file.main_rotor.indices,
            &helicopter_object_file.main_rotor.colors,
            &helicopter_object_file.main_rotor.normals,
        );
        let helicopter_tail_rotor_vao = create_vao(
            &helicopter_object_file.tail_rotor.vertices,
            &helicopter_object_file.tail_rotor.indices,
            &helicopter_object_file.tail_rotor.colors,
            &helicopter_object_file.tail_rotor.normals,
        );

        // Create Scene Graph Nodes
        let mut helicopter_node = SceneNode::new();
        let mut body_node =
            SceneNode::from_vao(helicopter_body_vao, helicopter_object_file.body.index_count);
        let mut door_node =
            SceneNode::from_vao(helicopter_door_vao, helicopter_object_file.door.index_count);
        let mut main_rotor_node = SceneNode::from_vao(
            helicopter_main_rotor_vao,
            helicopter_object_file.main_rotor.index_count,
        );
        let mut tail_rotor_node = SceneNode::from_vao(
            helicopter_tail_rotor_vao,
            helicopter_object_file.tail_rotor.index_count,
        );

        // Set the reference points for each helicopter body part
        helicopter_node.reference_point = glm::vec3(0.00, 0.00, 0.00);
        body_node.reference_point = glm::vec3(0.00, 0.00, 0.00);
        door_node.reference_point = glm::vec3(0.00, 0.00, 0.00);
        main_rotor_node.reference_point = glm::vec3(0.00, 0.00, 0.00);
        tail_rotor_node.reference_point = glm::vec3(0.35, 2.30, 10.40);

        // Set the helicopter starting position and orientation
        helicopter_node.position = *starting_position;
        helicopter_node.rotation = *starting_rotation;

        // Add helicopter body parts (child nodes) to a single parent node (body node)
        let helicopter_body_parts = vec![
            &mut body_node,
            &mut door_node,
            &mut main_rotor_node,
            &mut tail_rotor_node,
        ];
        for helicopter_part in &helicopter_body_parts {
            helicopter_node.add_child(helicopter_part);
        }

        // Return the helicopter parent and child nodes
        let mut helicopter = Helicopter {
            helicopter_node,
            body_node,
            door_node,
            main_rotor_node,
            tail_rotor_node,
        };

        return helicopter;
    }

    fn spin_rotors(
        &mut self,
        main_rotor_orientation: glm::Vec3,
        tail_rotor_orientation: glm::Vec3,
    ) {
        self.main_rotor_node.rotation = main_rotor_orientation;
        self.tail_rotor_node.rotation = tail_rotor_orientation;
    }

    fn animate_helicopter(&mut self, elapsed: f32, time_offset: f32) {
        // Update helicaopter heading
        let heading: toolbox::Heading = toolbox::simple_heading_animation(elapsed + time_offset);
        self.helicopter_node.position.x = heading.x;
        self.helicopter_node.position.z = heading.z;
        self.helicopter_node.rotation = glm::vec3(heading.pitch, heading.yaw, heading.roll);

        // Spin helicopter rotors
        self.spin_rotors(
            glm::Vec3::new(0.0, 8.0 * elapsed, 0.0),
            glm::Vec3::new(8.0 * elapsed, 0.0, 0.0),
        );
    }
}

/*********************************************************************/
/* Main Loop */
/*********************************************************************/
fn main() {
    // Set up the necessary objects to deal with windows and event handling
    let el = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_title("Gloom-rs")
        .with_resizable(true)
        .with_inner_size(glutin::dpi::LogicalSize::new(
            INITIAL_SCREEN_W,
            INITIAL_SCREEN_H,
        ));
    let cb = glutin::ContextBuilder::new().with_vsync(true);
    let windowed_context = cb.build_windowed(wb, &el).unwrap();
    // Uncomment these if you want to use the mouse for controls, but want it to be confined to the screen and/or invisible.
    // windowed_context.window().set_cursor_grab(true).expect("failed to grab cursor");
    // windowed_context.window().set_cursor_visible(false);

    // Set up a shared vector for keeping track of currently pressed keys
    let arc_pressed_keys = Arc::new(Mutex::new(Vec::<VirtualKeyCode>::with_capacity(10)));
    // Make a reference of this vector to send to the render thread
    let pressed_keys = Arc::clone(&arc_pressed_keys);

    // Set up shared tuple for tracking mouse movement between frames
    let arc_mouse_delta = Arc::new(Mutex::new((0f32, 0f32)));
    // Make a reference of this tuple to send to the render thread
    let mouse_delta = Arc::clone(&arc_mouse_delta);

    // Set up shared tuple for tracking changes to the window size
    let arc_window_size = Arc::new(Mutex::new((INITIAL_SCREEN_W, INITIAL_SCREEN_H, false)));
    // Make a reference of this tuple to send to the render thread
    let window_size = Arc::clone(&arc_window_size);

    // Spawn a separate thread for rendering, so event handling doesn't block rendering
    let render_thread = thread::spawn(move || {
        // Acquire the OpenGL Context and load the function pointers.
        // This has to be done inside of the rendering thread, because
        // an active OpenGL context cannot safely traverse a thread boundary
        let context = unsafe {
            let c = windowed_context.make_current().unwrap();
            gl::load_with(|symbol| c.get_proc_address(symbol) as *const _);
            c
        };

        let mut window_aspect_ratio = INITIAL_SCREEN_W as f32 / INITIAL_SCREEN_H as f32;

        // Set up openGL
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LESS);
            gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::MULTISAMPLE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
            gl::DebugMessageCallback(Some(util::debug_callback), ptr::null());

            // Print some diagnostics
            println!(
                "{}: {}",
                util::get_gl_string(gl::VENDOR),
                util::get_gl_string(gl::RENDERER)
            );
            println!("OpenGL\t: {}", util::get_gl_string(gl::VERSION));
            println!(
                "GLSL\t: {}",
                util::get_gl_string(gl::SHADING_LANGUAGE_VERSION)
            );
        }

        /*********************************************************************/
        /* Task 1: Load the Lunar Surface Model and Create a VAO for it */
        /*********************************************************************/
        let mut lunar_surface: mesh::Mesh = mesh::Terrain::load("./resources/lunarsurface.obj");
        let mut lunar_surface_vao = unsafe {
            create_vao(
                &lunar_surface.vertices,
                &lunar_surface.indices,
                &lunar_surface.colors,
                &lunar_surface.normals,
            )
        };

        // Projective/perspective Matrix
        let projective_matrix: glm::Mat4 = glm::perspective(
            (INITIAL_SCREEN_W as f32) / (INITIAL_SCREEN_H as f32),
            (60 as f32).to_radians(),
            1.0 as f32,
            1000.0 as f32,
        );

        // Translate origin of the models to the negative z range
        let view_matrix: glm::Mat4 = glm::mat4(
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, -75.0, //
            0.0, 0.0, 0.0, 1.0, //
        );

        /*********************************************************************/
        /* Task 2: Helicopter Parenting */
        /*********************************************************************/
        let mut helicopter = mesh::Helicopter::load("./resources/helicopter.obj");
        let mut root_node = scene_graph::SceneNode::new();
        let mut lunar_surface_node =
            SceneNode::from_vao(lunar_surface_vao, lunar_surface.index_count);

        let mut helicopter_1 = unsafe {
            Helicopter::new(
                &helicopter,
                &glm::Vec3::new(0.0, 0.0, 50.0),
                &glm::vec3(0.0, 0.7, 0.4),
            )
        };

        /*********************************************************************/
        /* Task 6: Animate At least 5 helicopters */
        /*********************************************************************/
        let mut helicopter_2 = unsafe {
            Helicopter::new(
                &helicopter,
                &glm::Vec3::new(0.0, 0.0, 0.0),
                &glm::vec3(0.0, 0.7, 0.4),
            )
        };

        let mut helicopter_3 = unsafe {
            Helicopter::new(
                &helicopter,
                &glm::Vec3::new(0.0, 0.0, 0.0),
                &glm::vec3(0.0, 0.7, 0.4),
            )
        };

        let mut helicopter_4 = unsafe {
            Helicopter::new(
                &helicopter,
                &glm::Vec3::new(0.0, 0.0, 0.0),
                &glm::vec3(0.0, 0.7, 0.4),
            )
        };

        let mut helicopter_5 = unsafe {
            Helicopter::new(
                &helicopter,
                &glm::Vec3::new(0.0, 10.0, 0.0),
                &glm::vec3(0.0, 0.7, 0.4),
            )
        };

        let mut helicopter_6 = unsafe {
            Helicopter::new(
                &helicopter,
                &glm::Vec3::new(0.0, 0.0, 0.0),
                &glm::vec3(0.0, 0.7, 0.4),
            )
        };

        let mut helicopter_7 = unsafe {
            Helicopter::new(
                &helicopter,
                &glm::Vec3::new(0.0, 0.0, 0.0),
                &glm::vec3(0.0, 0.7, 0.4),
            )
        };

        let mut helicopter_8 = unsafe {
            Helicopter::new(
                &helicopter,
                &glm::Vec3::new(0.0, 10.0, 0.0),
                &glm::vec3(0.0, 0.7, 0.4),
            )
        };

        lunar_surface_node.add_child(&helicopter_1.helicopter_node);
        lunar_surface_node.add_child(&helicopter_2.helicopter_node);
        lunar_surface_node.add_child(&helicopter_3.helicopter_node);
        lunar_surface_node.add_child(&helicopter_4.helicopter_node);
        lunar_surface_node.add_child(&helicopter_5.helicopter_node);
        lunar_surface_node.add_child(&helicopter_6.helicopter_node);
        lunar_surface_node.add_child(&helicopter_7.helicopter_node);
        lunar_surface_node.add_child(&helicopter_8.helicopter_node);

        root_node.add_child(&lunar_surface_node);

        /*********************************************************************/
        /* Main loop functions */
        /*********************************************************************/
        // Camera position and orientation
        let mut x: f32 = 0.0;
        let mut y: f32 = 0.0;
        let mut z: f32 = 0.0;
        let mut pitch: f32 = 0.0; // rotate about x
        let mut yaw: f32 = 0.0; // rotate about y

        // == // Set up your shaders here
        let shaders = unsafe {
            shader::ShaderBuilder::new()
                .attach_file("./shaders/simple.vert")
                .attach_file("./shaders/simple.frag")
                .link()
                .activate();
        };

        // The main rendering loop
        let first_frame_time = std::time::Instant::now();
        let mut previous_frame_time = first_frame_time;

        loop {
            // Compute time passed since the previous frame and since the start of the program
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(first_frame_time).as_secs_f32();
            let delta_time = now.duration_since(previous_frame_time).as_secs_f32();
            previous_frame_time = now;

            // Handle resize events
            if let Ok(mut new_size) = window_size.lock() {
                if new_size.2 {
                    context.resize(glutin::dpi::PhysicalSize::new(new_size.0, new_size.1));
                    window_aspect_ratio = new_size.0 as f32 / new_size.1 as f32;
                    (*new_size).2 = false;
                    println!("Window was resized to {}x{}", new_size.0, new_size.1);
                    unsafe {
                        gl::Viewport(0, 0, new_size.0 as i32, new_size.1 as i32);
                    }
                }
            }

            // Camera speed factor
            let translate_camera_speed = 25.0;
            let rotate_camera_speed = 1.0;

            // Handle keyboard input
            if let Ok(keys) = pressed_keys.lock() {
                for key in keys.iter() {
                    match key {
                        // The `VirtualKeyCode` enum is defined here:
                        //    https://docs.rs/winit/0.25.0/winit/event/enum.VirtualKeyCode.html
                        VirtualKeyCode::A => {
                            x += delta_time * translate_camera_speed;
                        }
                        VirtualKeyCode::D => {
                            x -= delta_time * translate_camera_speed;
                        }
                        VirtualKeyCode::S => {
                            y += delta_time * translate_camera_speed;
                        }
                        VirtualKeyCode::W => {
                            y -= delta_time * translate_camera_speed;
                        }
                        VirtualKeyCode::LControl => {
                            z += delta_time * translate_camera_speed;
                        }
                        VirtualKeyCode::LShift => {
                            z -= delta_time * translate_camera_speed;
                        }
                        VirtualKeyCode::Up => {
                            pitch += delta_time * rotate_camera_speed;
                        }
                        VirtualKeyCode::Down => {
                            pitch -= delta_time * rotate_camera_speed;
                        }
                        VirtualKeyCode::Left => {
                            yaw += delta_time * rotate_camera_speed;
                        }
                        VirtualKeyCode::Right => {
                            yaw -= delta_time * rotate_camera_speed;
                        }

                        _ => {}
                    }
                }
            }
            // Handle mouse movement. delta contains the x and y movement of the mouse since last frame in pixels
            if let Ok(mut delta) = mouse_delta.lock() {
                // == // Optionally access the accumulated mouse movement between
                // == // frames here with `delta.0` and `delta.1`

                *delta = (0.0, 0.0); // reset when done
            }

            /*********************************************************************/
            /* Camera transforms */
            /*********************************************************************/
            // Limit camera motion
            let min_pitch: f32 = (0.0 as f32).to_radians();
            let max_pitch: f32 = (180.0 as f32).to_radians();
            if pitch > max_pitch {
                pitch = max_pitch;
            }
            if pitch < min_pitch {
                pitch = min_pitch;
            }

            let mut transforms_matrix: glm::Mat4 = glm::identity();
            let mut transformation_so_far: glm::Mat4 = glm::identity();
            transforms_matrix = projective_matrix * view_matrix;

            transforms_matrix = glm::translate(&transforms_matrix, &glm::vec3(x, y, z));
            transforms_matrix = glm::rotate_x(&transforms_matrix, pitch);
            transforms_matrix = glm::rotate_y(&transforms_matrix, yaw);

            /*********************************************************************/
            /* Task 4a - Spin Helicopter Rotors */
            /*********************************************************************/
            // helicopter_1.spin_rotors(
            //     glm::Vec3::new(0.0, 5.0 * elapsed, 0.0),
            //     glm::Vec3::new(5.0 * elapsed, 0.0, 0.0),
            // );

            /*********************************************************************/
            /* Task 4b and 6 - Animate  Helicopter */
            /*********************************************************************/
            helicopter_1.animate_helicopter(elapsed, 0.0);
            helicopter_2.animate_helicopter(elapsed, 0.0);
            helicopter_3.animate_helicopter(elapsed, 0.75);
            helicopter_4.animate_helicopter(elapsed, 1.50);
            helicopter_5.animate_helicopter(elapsed, 1.50);
            helicopter_6.animate_helicopter(elapsed, 2.25);
            helicopter_7.animate_helicopter(elapsed, 3.0);
            helicopter_8.animate_helicopter(elapsed, 3.0);

            unsafe {
                // Clear the color and depth buffers
                gl::ClearColor(0.035, 0.046, 0.078, 1.0); // night sky, full opacity
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

                // == // Issue the necessary gl:: commands to draw your scene here
                draw_scene(&root_node, &transforms_matrix, &transformation_so_far);
            }

            // Display the new color buffer on the display
            context.swap_buffers().unwrap(); // we use "double buffering" to avoid artifacts
        }
    });

    // == //
    // == // From here on down there are only internals.
    // == //

    // Keep track of the health of the rendering thread
    let render_thread_healthy = Arc::new(RwLock::new(true));
    let render_thread_watchdog = Arc::clone(&render_thread_healthy);
    thread::spawn(move || {
        if !render_thread.join().is_ok() {
            if let Ok(mut health) = render_thread_watchdog.write() {
                println!("Render thread panicked!");
                *health = false;
            }
        }
    });

    // Start the event loop -- This is where window events are initially handled
    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        // Terminate program if render thread panics
        if let Ok(health) = render_thread_healthy.read() {
            if *health == false {
                *control_flow = ControlFlow::Exit;
            }
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(physical_size),
                ..
            } => {
                println!(
                    "New window size received: {}x{}",
                    physical_size.width, physical_size.height
                );
                if let Ok(mut new_size) = arc_window_size.lock() {
                    *new_size = (physical_size.width, physical_size.height, true);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            // Keep track of currently pressed keys to send to the rendering thread
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: key_state,
                                virtual_keycode: Some(keycode),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                if let Ok(mut keys) = arc_pressed_keys.lock() {
                    match key_state {
                        Released => {
                            if keys.contains(&keycode) {
                                let i = keys.iter().position(|&k| k == keycode).unwrap();
                                keys.remove(i);
                            }
                        }
                        Pressed => {
                            if !keys.contains(&keycode) {
                                keys.push(keycode);
                            }
                        }
                    }
                }

                // Handle Escape and Q keys separately
                match keycode {
                    Escape => {
                        *control_flow = ControlFlow::Exit;
                    }
                    Q => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                }
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                // Accumulate mouse movement
                if let Ok(mut position) = arc_mouse_delta.lock() {
                    *position = (position.0 + delta.0 as f32, position.1 + delta.1 as f32);
                }
            }
            _ => {}
        }
    });
}
