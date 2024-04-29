use cgmath::*;
use winit::keyboard::{Key, NamedKey};
use std::f32::consts::PI;
use std::time::Duration;
use winit::dpi::PhysicalPosition;
use winit::event::*;
use crate::structs::ShaderConfig;
/// Represents a camera in 3D space.
///
/// The camera has a position and a rotation. The position is a point in 3D space, and the rotation is a quaternion that represents the orientation of the camera.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub position: Point3<f32>,
    pub rotation: Quaternion<f32>,
}

impl Camera {
    pub fn new<V: Into<Point3<f32>>, Y: Into<Rad<f32>> + std::marker::Copy, P: Into<Rad<f32>> + std::marker::Copy>(
        position: V,
        yaw: Y,
        pitch: P,
    ) -> Self {
        let quaternion = Quaternion::from_angle_y(yaw) * Quaternion::from_angle_x(pitch);
        println!("Camera initial roation quaternion = {:?}", quaternion);
        Self {
            position: position.into(),
            rotation: quaternion,
        }
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(self.position, self.position + self.rotation.rotate_vector(Vector3::unit_z()), Vector3::unit_y())
    }
}

/// Represents a projection of a 3D scene onto the 2D plane of the camera.
///
/// The projection is defined by an aspect ratio, a field of view, and near and far clipping planes.
pub struct Projection {
    aspect: f32,
    pub fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<Rad<f32>>>(width: u32, height: u32, fovy: F, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

/// Controls the movement and rotation of a camera.
///
/// The controller keeps track of the amount of movement in each direction (left, right, forward, backward, up, down), the amount of rotation (horizontal and vertical), and the amount of scrolling.
/// It also has a speed and a sensitivity, which control how fast the camera moves and how sensitive it is to rotation.
#[derive(Debug)]
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, key: &Key, state: &ElementState) -> bool {
        let amount = if state == &ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {            
            Key::Character(c) if c.to_lowercase() == "w" => {
                self.amount_forward = amount;
                true
            }
            Key::Character(c) if c.to_lowercase() == "s" => {
                self.amount_backward = amount;
                true
            }
            Key::Character(c) if c.to_lowercase() == "a" => {
                self.amount_left = amount;
                true
            }
            Key::Character(c) if c.to_lowercase() == "d" => {
                self.amount_right = amount;
                true
            }
            Key::Named(NamedKey::ArrowUp) => {
                self.amount_forward = amount;
                true
            }
            Key::Named(NamedKey::ArrowDown) => {
                self.amount_backward = amount;
                true
            }
            Key::Named(NamedKey::ArrowLeft) => {
                self.amount_left = amount;
                true
            }
            Key::Named(NamedKey::ArrowRight) => {
                self.amount_right = amount;
                true
            }
            Key::Named(NamedKey::Space) => {
                self.amount_up = amount;
                true
            }
            Key::Named(NamedKey::Shift) => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = -mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = match delta {
            // I'm assuming a line is about 100 pixels
            MouseScrollDelta::LineDelta(_, scroll) => -scroll * 0.5,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => -*scroll as f32,
        };
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();

        // Move forward/backward and left/right
        let forward = camera.rotation.rotate_vector(Vector3::new(0.0, 0.0, -1.0)).normalize();
        let right = camera.rotation.rotate_vector(Vector3::new(1.0, 0.0, 0.0)).normalize();
        camera.position += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
        camera.position += right * (self.amount_right - self.amount_left) * self.speed * dt;

        // Move up/down
        camera.position.y += (self.amount_up - self.amount_down) * self.speed * dt;
        

        // Rotate using quaternion
        let camera_pitch = Euler::from(camera.rotation).x;
        let pitch_quaternion = Quaternion::from_axis_angle(Vector3::unit_x(), Rad(-self.rotate_vertical) * self.sensitivity * dt);
        let yaw_quaternion = Quaternion::from_axis_angle(Vector3::unit_y(), Rad(self.rotate_horizontal) * self.sensitivity * dt);

        // Combine pitch and yaw rotations using quaternion multiplication
        // if camera_pitch > Rad(PI * 0.5) && self.rotate_vertical > 0.0 {
        //     camera.rotation = yaw_quaternion * camera.rotation;
        // } else if camera_pitch < Rad(-PI * 0.5) && self.rotate_vertical < 0.0 {
        //     camera.rotation = yaw_quaternion * camera.rotation;
        // } else {
        camera.rotation = yaw_quaternion * camera.rotation * pitch_quaternion;
        // }

        // Keep the camera's angle from going too high/low.
        println!("Camera x = {:?}", Euler::from(camera.rotation));

        // Reset rotation values
        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        // Update the scroll value if you want to use it for zooming
        self.scroll = 0.0;
    }
}