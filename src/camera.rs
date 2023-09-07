use cgmath::*;
use std::f32::consts::FRAC_PI_2;
use std::time::Duration;
use winit::dpi::PhysicalPosition;
use winit::event::*;
use cgmath::{Quaternion, Vector3, Matrix4, Point3, Rad, Deg, InnerSpace, Rotation3, Rotation};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;


#[repr(C)]
#[derive(Debug)]
pub struct Camera {
    position: Point3<f32>,
    orientation: Quaternion<f32>,
}

impl Camera {
    pub fn new(position: Point3<f32>) -> Self {
        Camera {
            position,
            orientation: Quaternion::from_angle_y(Rad(0.0)),
        }
    }

    pub fn get_view_matrix(&self) -> Matrix4<f32> {
        let rotation_matrix = Matrix4::from(self.orientation);
        let translation_matrix = Matrix4::from_translation(-self.position.to_vec());
        rotation_matrix * translation_matrix
    }

    pub fn move_forward(&mut self, distance: f32) {
        let forward = self.orientation * Vector3::unit_z();
        self.position += forward * distance;
    }

    pub fn move_backward(&mut self, distance: f32) {
        let forward = self.orientation * -Vector3::unit_z();
        self.position += forward * distance;
    }

    pub fn strafe_right(&mut self, distance: f32) {
        let right = self.orientation * Vector3::unit_x();
        self.position += right * distance;
    }

    pub fn strafe_left(&mut self, distance: f32) {
        let right = self.orientation * -Vector3::unit_x();
        self.position += right * distance;
    }

    pub fn rotate(&mut self, yaw: f32, pitch: f32) {
        let yaw_quat = Quaternion::from_angle_y(Rad(yaw));
        let pitch_quat = Quaternion::from_angle_x(Rad(pitch));

        // Combine the yaw and pitch rotations using quaternion multiplication
        self.orientation = self.orientation * yaw_quat * pitch_quat;
    }

    pub fn forward(&self) -> Vector3<f32> {
        self.orientation * -Vector3::unit_z()
    }

    pub fn move_locally(&mut self, offset: Vector3<f32>) {
        self.position += offset;
    }
}



#[repr(C)]
#[derive(Debug)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraController {
    amount_right: f32,
    amount_forward: f32,
    amount_up: f32,
    yaw: f32,
    pitch: f32,
    speed: f32,
    sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_up: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.amount_forward = amount;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.amount_forward = -amount;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.amount_right = -amount;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.amount_right = amount;
                true
            }
            VirtualKeyCode::Space => {
                self.amount_up = amount;
                true
            }
            VirtualKeyCode::LShift => {
                self.amount_up = -amount;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.yaw -= mouse_dx as f32 * self.sensitivity;
        self.pitch -= mouse_dy as f32 * self.sensitivity;

        // Clamp pitch to prevent flipping the camera
        self.pitch = self.pitch.max(-89.0).min(89.0);
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let speed = self.speed * dt.as_secs_f32();

        let forward = camera.forward();
        let right = Vector3::new(-forward.z, 0.0, forward.x).normalize();

        let movement = forward * self.amount_forward + right * self.amount_right;

        camera.move_locally(movement * speed);

        self.amount_forward = 0.0;
        self.amount_right = 0.0;
        self.amount_up = 0.0;

        camera.rotate(self.yaw, -self.pitch);
        self.yaw = 0.0;
        self.pitch = 0.0;
    }
}

#[repr(C)]
#[derive(Debug)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    position: [f32; 3],
    direction: [f32; 3],
}

impl CameraUniform {
    pub fn new(camera: &Camera) -> Self {
        Self {
            position: camera.position.into(),
            direction: camera.forward().into(),
        }
    }
}