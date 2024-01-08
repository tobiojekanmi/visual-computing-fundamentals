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
unsafe fn create_vao(vertices: &Vec<f32>, indices: &Vec<u32>) -> u32 {
    // * Generate a VAO and bind it
    let num_vao = 1;
    let mut vao_id: u32 = 0;
    gl::GenVertexArrays(num_vao, &mut vao_id);
    gl::BindVertexArray(vao_id);

    // * Generate a VBO and bind it
    let num_vbo = 1;
    let mut vbo_id: u32 = 0;
    gl::GenBuffers(num_vbo, &mut vbo_id);
    gl::BindBuffer(gl::ARRAY_BUFFER, vbo_id);

    // * Fill it with data
    gl::BufferData(
        gl::ARRAY_BUFFER,
        byte_size_of_array(vertices),
        pointer_to_array(vertices),
        gl::STATIC_DRAW,
    );

    // * Configure a VAP for the data and enable it
    gl::EnableVertexAttribArray(0);
    gl::VertexAttribPointer(
        0,           // Index of the generic vertex attribute to modify
        3, // Number of vertex attributes (i.e., 2 and 3 for [x,y] and [x, y, z] respectively)
        gl::FLOAT, // Data type of each component in the array
        gl::FALSE, // OpenGL should not normalize the values in the buffer
        0, // let OpenGL infer the correct stride attributes
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

        // == // Set up your VAO around here

        // TASK 1 - indices 1
        // let vertices: Vec<f32> = vec![
        //     -0.85, 0.30, 0.00, // Vert 0
        //     -0.35, 0.30, 0.00, // Vert 1
        //     -0.25, 0.30, 0.00, // Vert 2
        //     0.25, 0.30, 0.00, // Vert 3
        //     0.35, 0.30, 0.00, // Vert 4
        //     0.85, 0.30, 0.00, // Vert 5
        //     -0.6, -0.30, 0.00, // Vert 6
        //     0.00, -0.30, 0.00, // Vert 7
        //     0.6, -0.30, 0.00, // Vert 8
        //     -0.30, 0.30, 0.00, // Vert 9
        //     0.30, 0.30, 0.00, // Vert 10
        //     -0.55, -0.30, 0.00, // Vert 11
        //     -0.05, -0.30, 0.00, // Vert 12
        //     0.05, -0.30, 0.00, // Vert 13
        //     0.55, -0.30, 0.00, // Vert 14
        // ];

        // let indices: Vec<u32> = vec![
        //     6, 1, 0, // Upper Left
        //     3, 2, 7, // Upper Middle
        //     5, 4, 8, // Upper Right
        //     11, 12, 9, // Lower Left
        //     13, 14, 10, // Lower Right
        // ];

        // TASK 1 - indices 2
        // let vertices: Vec<f32> = vec![
        //     -0.10, 0.05, 0.00, // Vert 0
        //     -0.50, 0.85, 0.00, // Vert 1
        //     -0.90, 0.05, 0.00, // Vert 2
        //     0.90, 0.05, 0.00, // Vert 3
        //     0.50, 0.85, 0.00, // Vert 4
        //     0.10, 0.05, 0.00, // Vert 5
        //     0.40, 0.85, 0.00, // Vert 6
        //     -0.40, 0.85, 0.00, // Vert 7
        //     0.00, 0.05, 0.00, // Vert 8
        //     -0.50, -0.85, 0.00, // Vert 1
        //     -0.10, -0.05, 0.00, // Vert 0
        //     -0.90, -0.05, 0.00, // Vert 2
        //     0.90, -0.05, 0.00, // Vert 3
        //     0.10, -0.05, 0.00, // Vert 5
        //     0.50, -0.85, 0.00, // Vert 4
        //     0.40, -0.85, 0.00, // Vert 6
        //     0.00, -0.05, 0.00, // Vert 8
        //     -0.40, -0.85, 0.00, // Vert 7
        // ];

        // let indices: Vec<u32> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17];

        // TASK 2A - Given Vertices
        // let vertices: Vec<f32> = vec![
        //     0.60, -0.80, -1.20, // Vert 0
        //     0.00, 0.40, 0.00, // Vert 1
        //     -0.80, -0.20, 1.20, // Vert 2
        // ];
        // let indices: Vec<u32> = vec![0, 1, 2];

        // TASK 2A - Modified Vertices
        // let vertices: Vec<f32> = vec![
        //     0.60, -0.80, -1.00, // Vert 0
        //     0.00, 0.40, 0.00, // Vert 1
        //     -0.80, -0.20, 1.00, // Vert 2
        // ];
        // let indices: Vec<u32> = vec![0, 1, 2];

        // TASK 2B
        // let vertices: Vec<f32> = vec![
        //     0.95, -0.60, 0.00, // Vert 0
        //     0.35, 0.60, 0.00, // Vert 1
        //     -0.25, -0.60, 0.00, // Vert 2
        //     -0.35, -0.60, 0.00, //Vert 3
        //     0.25, 0.60, 0.00, // vert 4
        //     -0.95, 0.60, 0.00,
        // ];
        // let indices: Vec<u32> = vec![0, 1, 2, 3, 4, 5];

        // TASK 2D
        let vertices: Vec<f32> = vec![
            0.65, -0.60, 0.00, // Vert 0
            0.05, 0.60, 0.00, // Vert 1
            -0.55, -0.60, 0.00, // Vert 2
            -0.6, -0.60, 0.00, //Vert 3
            -0.0, 0.60, 0.00, // vert 4
            -0.6, 0.60, 0.00,
        ];
        let indices: Vec<u32> = vec![0, 1, 2, 3, 4, 5];

        // OPTIONAL TASK 2 - Draw a circle
        // let mut vertices: Vec<f32> = Vec::new();
        // let mut indices: Vec<u32> = Vec::new();
        // for step in 0..1002 {
        //     let mut angle = (360 * step / 1000) as f32;

        //    // Compute the x and y coordinates and normalize using normalized
        //    // screen height and width values (to fit a circle and not an oval)
        //     let mut x = 0.6 * angle.to_radians().cos();
        //     let mut y = 0.8 * angle.to_radians().sin();
        //     let mut z = 0.0;

        //     vertices.extend_from_slice(&[x, y, z]);

        //     if step >= 1 {
        //         // Case 1: Triangle Primitives
        //         // indices.extend_from_slice(&[0, step - 1, step]);

        //         // Case 2: Line Primitives
        //         indices.extend_from_slice(&[step - 1, step]);
        //     }
        // }

        // OPTIONAL TASK 3 - Draw a spiral
        // let mut radius: f32 = 0.0;
        // let mut vertices: Vec<f32> = Vec::new();
        // let mut indices: Vec<u32> = Vec::new();
        // for step in 0..10002 {
        //     let mut angle = (360 * step / 1000) as f32;

        //     // Compute the x and y coordinates and normalize using normalized
        //     // screen height and width values (to fit a circular and not oval spiral)
        //     let mut x = 0.6 * angle.to_radians().cos() * radius;
        //     let mut y = 0.8 * angle.to_radians().sin() * radius;
        //     let z = 0.0;

        //     vertices.extend_from_slice(&[x, y, z]);

        //     radius += 0.0001;

        //     if step >= 1 {
        //         indices.extend_from_slice(&[step - 1, step]);
        //     }
        // }

        // OPTIONAL TASK 7 - Draw Sine and Cosine Graphs
        // let mut vertices: Vec<f32> = Vec::new();
        // let mut indices: Vec<u32> = Vec::new();
        // for step in 0..10002 {
        //     let mut angle = (360 * step / 1000) as f32;
        //     let mut x: f32 = 0.6 * ((step as f32 - 5000.0) / 1002.0);
        //     let mut y = 0.8 * angle.to_radians().sin();
        //     // let mut y = 0.8 * angle.to_radians().cos();
        //     let mut z = 0.0;

        //     vertices.extend_from_slice(&[x, y, z]);
        //     if step >= 1 {
        //         indices.extend_from_slice(&[step - 1, step]);
        //     }
        // }

        let my_vao = unsafe { create_vao(&vertices, &indices) };

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

        // Used to demonstrate keyboard handling for exercise 2.
        let mut _arbitrary_number = 0.0; // feel free to remove

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

            // Handle keyboard input
            if let Ok(keys) = pressed_keys.lock() {
                for key in keys.iter() {
                    match key {
                        // The `VirtualKeyCode` enum is defined here:
                        //    https://docs.rs/winit/0.25.0/winit/event/enum.VirtualKeyCode.html
                        VirtualKeyCode::A => {
                            _arbitrary_number += delta_time;
                        }
                        VirtualKeyCode::D => {
                            _arbitrary_number -= delta_time;
                        }

                        // default handler:
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

            unsafe {
                // Clear the color and depth buffers
                gl::ClearColor(0.035, 0.046, 0.078, 1.0); // night sky, full opacity
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

                // == // Issue the necessary gl:: commands to draw your scene here
                let num_elements = indices.len() as i32;
                gl::BindVertexArray(my_vao);

                // Line Primitives Rendering
                // gl::DrawElements(gl::LINES, num_elements, gl::UNSIGNED_INT, ptr::null());

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
