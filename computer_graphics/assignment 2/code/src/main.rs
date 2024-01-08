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

mod shader;
mod util;

use glutin::event::{
    DeviceEvent,
    ElementState::{Pressed, Released},
    Event, KeyboardInput,
    VirtualKeyCode::{self, *},
    WindowEvent,
};
use glutin::event_loop::ControlFlow;

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

// == // Generate your VAO here
unsafe fn create_vao(vertices: &Vec<f32>, indices: &Vec<u32>, colors: &Vec<f32>) -> u32 {
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
            // gl::Enable(gl::CULL_FACE);
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

        // == // Set up your VAO around here
        // ---------------------------- TASK 1 ---------------------------- \\
        // let vertices: Vec<f32> = vec![
        //     -0.10, 0.05, 0.00, // Vertex 0
        //     -0.50, 0.85, 0.00, // Vertex 1
        //     -0.90, 0.05, 0.00, // Vertex 2
        //     0.90, 0.05, 0.00, // Vertex 3
        //     0.50, 0.85, 0.00, // Vertex 4
        //     0.10, 0.05, 0.00, // Vertex 5
        //     0.40, 0.85, 0.00, // Vertex 6
        //     -0.40, 0.85, 0.00, // Vertex 7
        //     0.00, 0.05, 0.00, // Vertex 8
        //     -0.50, -0.85, 0.00, // Vertex 9
        //     -0.10, -0.05, 0.00, // Vertex 10
        //     -0.90, -0.05, 0.00, // Vertex 11
        //     0.90, -0.05, 0.00, // Vertex 12
        //     0.10, -0.05, 0.00, // Vertex 13
        //     0.50, -0.85, 0.00, // Vertex 14
        //     0.40, -0.85, 0.00, // Vertex 15
        //     0.00, -0.05, 0.00, // Vertex 16
        //     -0.40, -0.85, 0.00, // Vertex 17
        // ];

        // let colors: Vec<f32> = vec![
        //     1.00, 0.00, 0.00, 1.00, // Vertex 0
        //     0.00, 1.00, 0.00, 1.00, // Vertex 1
        //     0.00, 0.00, 1.00, 1.00, // Vertex 2
        //     1.00, 0.00, 0.00, 1.00, // Vertex 3
        //     0.00, 1.00, 0.00, 1.00, // Vertex 4
        //     0.00, 0.00, 1.00, 1.00, // Vertex 5
        //     1.00, 0.00, 0.00, 1.00, // Vertex 6
        //     0.00, 1.00, 0.00, 1.00, // Vertex 7
        //     0.00, 0.00, 1.00, 1.00, // Vertex 8
        //     1.00, 0.00, 0.00, 1.00, // Vertex 9
        //     0.00, 1.00, 0.00, 1.00, // Vertex 10
        //     0.00, 0.00, 1.00, 1.00, // Vertex 11
        //     1.00, 0.00, 0.00, 1.00, // Vertex 12
        //     0.00, 1.00, 0.00, 1.00, // Vertex 13
        //     0.00, 0.00, 1.00, 1.00, // Vertex 14
        //     1.00, 0.00, 0.00, 1.00, // Vertex 15
        //     0.00, 1.00, 0.00, 1.00, // Vertex 16
        //     0.00, 0.00, 1.00, 1.00, // Vertex 17
        // ];

        // let indices: Vec<u32> = vec![
        //     0, 1, 2, // Triangle 1 - Upper Left
        //     3, 4, 5, // Triangle 2 - Upper Right
        //     6, 7, 8, // Triangle 3 - Upper Middle
        //     9, 10, 11, // Triangle 4 - Bottom Left
        //     12, 13, 14, // Triangle 5 - Bottom Right
        //     15, 16, 17, // Triangle 6 - Bottom Middle
        // ];

        // --------------------------- TASK 2A --------------------------- \\
        // let vertices: Vec<f32> = vec![
        //     0.95, 0.20, 0.90, // Vertex 0
        //     0.05, 0.90, 0.90, // Vertex 1
        //     -0.30, -0.60, 0.90, // Vertex 2
        //     -0.05, 0.90, 0.45, // Vertex 3
        //     -0.95, 0.20, 0.45, // Vertex 4
        //     0.30, -0.60, 0.45, // Vertex 5
        //     -0.50, -0.50, 0.00, // Vertex 6
        //     0.50, -0.50, 0.00, // Vertex 7
        //     0.00, 0.80, 0.00, // Vertex 8
        // ];

        // let colors: Vec<f32> = vec![
        //     1.00, 0.10, 0.10, 0.50, // Vertex 0
        //     1.00, 0.10, 0.10, 0.50, // Vertex 1
        //     1.00, 0.10, 0.10, 0.50, // Vertex 2
        //     0.10, 1.00, 0.10, 0.50, // Vertex 3
        //     0.10, 1.00, 0.10, 0.50, // Vertex 4
        //     0.10, 1.00, 0.10, 0.50, // Vertex 5
        //     0.10, 0.10, 1.00, 0.50, // Vertex 6
        //     0.10, 0.10, 1.00, 0.50, // Vertex 7
        //     0.10, 0.10, 1.00, 0.50, // Vertex 8
        // ];

        // let indices: Vec<u32> = vec![
        //     0, 1, 2, // Triangle 1
        //     3, 4, 5, // Triangle 2
        //     6, 7, 8, // Triangle 3
        // ];

        // ------------------------ TASK 2B - I ------------------------ \\
        // let vertices: Vec<f32> = vec![
        //     0.95, 0.20, 0.90, // Vertex 0
        //     0.05, 0.90, 0.90, // Vertex 1
        //     -0.30, -0.60, 0.90, // Vertex 2
        //     -0.05, 0.90, 0.45, // Vertex 3
        //     -0.95, 0.20, 0.45, // Vertex 4
        //     0.30, -0.60, 0.45, // Vertex 5
        //     -0.50, -0.50, 0.00, // Vertex 6
        //     0.50, -0.50, 0.00, // Vertex 7
        //     0.00, 0.80, 0.00, // Vertex 8
        // ];

        // let colors: Vec<f32> = vec![
        //     0.10, 0.10, 1.00, 0.50, // Vertex 6
        //     0.10, 0.10, 1.00, 0.50, // Vertex 7
        //     0.10, 0.10, 1.00, 0.50, // Vertex 8
        //     1.00, 0.10, 0.10, 0.50, // Vertex 0
        //     1.00, 0.10, 0.10, 0.50, // Vertex 1
        //     1.00, 0.10, 0.10, 0.50, // Vertex 2
        //     0.10, 1.00, 0.10, 0.50, // Vertex 3
        //     0.10, 1.00, 0.10, 0.50, // Vertex 4
        //     0.10, 1.00, 0.10, 0.50, // Vertex 5
        // ];

        // let indices: Vec<u32> = vec![
        //     0, 1, 2, // Triangle 1
        //     3, 4, 5, // Triangle 2
        //     6, 7, 8, // Triangle 3
        // ];

        // ------------------------ TASK 2B - II ------------------------ \\
        // let vertices: Vec<f32> = vec![
        //     0.95, 0.20, -0.90, // Vertex 0
        //     0.05, 0.90, -0.90, // Vertex 1
        //     -0.30, -0.60, -0.90, // Vertex 2
        //     -0.05, 0.90, -0.45, // Vertex 3
        //     -0.95, 0.20, -0.45, // Vertex 4
        //     0.30, -0.60, -0.45, // Vertex 5
        //     -0.50, -0.50, 0.00, // Vertex 6
        //     0.50, -0.50, 0.00, // Vertex 7
        //     0.00, 0.80, 0.00, // Vertex 8
        // ];

        // let colors: Vec<f32> = vec![
        //     0.10, 0.10, 1.00, 0.50, // Vertex 6
        //     0.10, 0.10, 1.00, 0.50, // Vertex 7
        //     0.10, 0.10, 1.00, 0.50, // Vertex 8
        //     1.00, 0.10, 0.10, 0.50, // Vertex 0
        //     1.00, 0.10, 0.10, 0.50, // Vertex 1
        //     1.00, 0.10, 0.10, 0.50, // Vertex 2
        //     0.10, 1.00, 0.10, 0.50, // Vertex 3
        //     0.10, 1.00, 0.10, 0.50, // Vertex 4
        //     0.10, 1.00, 0.10, 0.50, // Vertex 5
        // ];

        // let indices: Vec<u32> = vec![
        //     0, 1, 2, // Triangle 1
        //     3, 4, 5, // Triangle 2
        //     6, 7, 8, // Triangle 3
        // ];

        // ------------------------ TASK 3 ------------------------ \\
        // let vertices: Vec<f32> = vec![
        //     0.95, -0.60, 0.90, // Vert 0
        //     0.35, 0.60, 0.90, // Vert 1
        //     -0.25, -0.60, 0.90, // Vert 2
        //     -0.35, -0.60, 0.90, //Vert 3
        //     0.25, 0.60, 0.90, // vert 4
        //     -0.95, 0.60, 0.90,
        // ];

        // let colors: Vec<f32> = vec![
        //     1.00, 1.00, 0.00, 1.00, // Vertex 0
        //     1.00, 1.00, 0.00, 1.00, // Vertex 1
        //     1.00, 1.00, 0.00, 1.00, // Vertex 2
        //     0.00, 1.00, 1.00, 1.00, // Vertex 3
        //     0.00, 1.00, 1.00, 1.00, // Vertex 4
        //     0.00, 1.00, 1.00, 1.00, // Vertex 5
        // ];

        // let indices: Vec<u32> = vec![
        //     0, 1, 2, // Triangle 1
        //     3, 4, 5, // Triangle 2
        // ];

        /* Task 4*/
        let vertices: Vec<f32> = vec![
            0.95, -0.60, 0.00, // Vert 0
            0.35, 0.60, 0.00, // Vert 1
            -0.25, -0.60, 0.00, // Vert 2
            -0.35, -0.60, 0.00, //Vert 3
            0.25, 0.60, 0.00, // vert 4
            -0.95, 0.60, 0.00,
        ];

        let colors: Vec<f32> = vec![
            1.00, 1.00, 0.00, 1.00, // Vertex 0
            1.00, 1.00, 0.00, 1.00, // Vertex 1
            1.00, 1.00, 0.00, 1.00, // Vertex 2
            0.00, 1.00, 1.00, 1.00, // Vertex 3
            0.00, 1.00, 1.00, 1.00, // Vertex 4
            0.00, 1.00, 1.00, 1.00, // Vertex 5
        ];

        let indices: Vec<u32> = vec![
            0, 1, 2, // Triangle 1
            3, 4, 5, // Triangle 2
        ];

        // Projective/perspective Matrix
        let projective_matrix: glm::Mat4 = glm::perspective(
            (INITIAL_SCREEN_W as f32) / (INITIAL_SCREEN_H as f32), // aspect ratio -> width/height
            (45 as f32).to_radians(),                              // 45 degrees FOV in radians
            1.0 as f32,                                            // near clipping plane
            100.0 as f32,                                          // far clipping plane
        );

        // Translate triangles into the negative z range
        let view_matrix: glm::Mat4 = glm::mat4(
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, -3.0, //
            0.0, 0.0, 0.0, 1.0, //
        );

        // Camera position
        let mut x: f32 = 0.0;
        let mut y: f32 = 0.0;
        let mut z: f32 = 0.0;
        let mut pitch: f32 = 0.0; // rotate about x
        let mut yaw: f32 = 0.0; // rotate about y

        let my_vao = unsafe { create_vao(&vertices, &indices, &colors) };

        // == // Set up your shaders here

        // Basic usage of shader helper:
        // The example code below creates a 'shader' object.
        // It which contains the field `.program_id` and the method `.activate()`.
        // The `.` in the path is relative to `Cargo.toml`.
        // This snippet is not enough to do the exercise, and will need to be modified (outside
        // of just using the correct path), but it only needs to be called once

        /*
        let simple_shader = unsafe {
            shader::ShaderBuilder::new()
                .attach_file("./path/to/simple/shader.file")
                .link()
        };
        */

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
            let translate_camera_speed = 1.0;
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

            // == // Please compute camera transforms here (exercise 2 & 3)
            let mut transforms_matrix: glm::Mat4 = glm::identity();
            transforms_matrix = projective_matrix * view_matrix;

            //* Task 4
            transforms_matrix = glm::translate(&transforms_matrix, &glm::vec3(x, y, z));
            transforms_matrix = glm::rotate_x(&transforms_matrix, pitch);
            transforms_matrix = glm::rotate_y(&transforms_matrix, yaw);

            //* Optional Task 1
            // let max_pitch: f32 = (89 as f32).to_radians();
            // if pitch > max_pitch {
            //     pitch = max_pitch;
            // }
            // if pitch < -max_pitch {
            //     pitch = -max_pitch;
            // }

            // transforms_matrix = glm::rotate_x(&transforms_matrix, pitch);
            // transforms_matrix = glm::rotate_y(&transforms_matrix, yaw);
            // transforms_matrix = glm::translate(&transforms_matrix, &glm::vec3(x, y, z));

            unsafe {
                // Task 3 (Affine Transformations)
                gl::Uniform1f(0, elapsed.sin());

                // Task 4
                gl::UniformMatrix4fv(1, 1, gl::FALSE, transforms_matrix.as_ptr());
            }

            unsafe {
                // Clear the color and depth buffers
                gl::ClearColor(0.035, 0.046, 0.078, 1.0); // night sky, full opacity
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

                // == // Issue the necessary gl:: commands to draw your scene here
                let num_elements = indices.len() as i32;
                gl::BindVertexArray(my_vao);

                // Triangle Primitives Rendering
                gl::DrawElements(gl::TRIANGLES, num_elements, gl::UNSIGNED_INT, ptr::null());
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
