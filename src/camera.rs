use cgmath::*;
use std::f32::consts::FRAC_PI_2;
use std::time::Duration;
use winit::dpi::PhysicalPosition;
use winit::event::*;

// #[rustfmt::skip]
// pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
//     1.0, 0.0, 0.0, 0.0,
//     0.0, 1.0, 0.0, 0.0,
//     0.0, 0.0, 0.5, 0.5,
//     0.0, 0.0, 0.0, 1.0,
// );

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;


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
        println!("Camera::new: Quaternion::from_angle_y(yaw) * Quaternion::from_angle_x(pitch) = {:?}", quaternion);
        Self {
            position: position.into(),
            rotation: quaternion,
        }
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(self.position, self.position + self.rotation.rotate_vector(Vector3::unit_z()), Vector3::unit_y())
    }
}

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
        // OPENGL_TO_WGPU_MATRIX * 
        perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

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
                self.amount_backward = amount;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.amount_left = amount;
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
        let pitch_quaternion = Quaternion::from_axis_angle(Vector3::unit_x(), Rad(-self.rotate_vertical) * self.sensitivity * dt);
        let yaw_quaternion = Quaternion::from_axis_angle(Vector3::unit_y(), Rad(self.rotate_horizontal) * self.sensitivity * dt);

        // Combine pitch and yaw rotations using quaternion multiplication
        camera.rotation = yaw_quaternion * camera.rotation * pitch_quaternion;

        // Keep the camera's angle from going too high/low.
        // if camera.quaternion.v.x < -SAFE_FRAC_PI_2 {
        //     camera.quaternion.v.x = -SAFE_FRAC_PI_2;
        // } else if camera.quaternion.v.x > SAFE_FRAC_PI_2 {
        //     camera.quaternion.v.x = SAFE_FRAC_PI_2;
        // }

        // Reset rotation values
        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        // Update the scroll value if you want to use it for zooming
        self.scroll = 0.0;
    }
}