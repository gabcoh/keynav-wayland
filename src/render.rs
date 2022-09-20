use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::rc::Rc;

use log::info;

#[derive(Clone)]
pub struct RenderManager {
    active_region: cairo::Rectangle,
    height: u32,
    width: u32,
    stride: i32,
    format: cairo::Format,
    temp: Rc<File>,
    cairo_context: cairo::Context,
}

impl RenderManager {
    pub fn init(format: cairo::Format, width: u32, height: u32) -> Result<Self, String> {
        let x = cairo::ImageSurface::create(format, 0, 0)
            .map_err(|_| "Failed to create temporary cairo surface")?;
        let mut renderer = RenderManager {
            active_region: cairo::Rectangle {
                width: 1.0,
                height: 1.0,
                x: 0.0,
                y: 0.0,
            },
            height: 0,
            width: 0,
            stride: -1,
            format: format,
            temp: Rc::new(
                tempfile::tempfile().map_err(|_| "Failed to create temporary backing file")?,
            ),
            cairo_context: cairo::Context::new(&x).map_err(|err| err.to_string())?,
        };
        renderer.set_bounds(width, height)?;
        Ok(renderer)
    }
    pub fn set_bounds(&mut self, width: u32, height: u32) -> Result<(), String> {
        self.stride = self
            .format
            .stride_for_width(width)
            .map_err(|_| "Failed to calculate [format.stride_for_width]")?;
        self.width = width;
        self.height = height;
        self.temp
            .set_len(self.get_buf_size() as u64)
            .map_err(|_| "Failed to set length of temporary backing file")?;

        let buf = unsafe {
            memmap::MmapOptions::new()
                .len(self.get_buf_size() as usize)
                .map(&self.temp)
                .unwrap()
        }
        .make_mut()
        .expect("mmap tempfile");
        let cairo_surface = cairo::ImageSurface::create_for_data(
            buf,
            self.get_format(),
            self.get_width() as i32,
            self.get_height() as i32,
            self.get_stride(),
        )
        .expect("Create cairo image surface");
        self.cairo_context =
            cairo::Context::new(&cairo_surface).expect("Get cairo context from surface");
        self.cairo_context
            .scale(self.get_width() as f64, self.get_height() as f64);
        self.cairo_context.set_operator(cairo::Operator::Source);

        self.redraw()
    }
    pub fn redraw(&self) -> Result<(), String> {
        self.cairo_context.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        self.cairo_context.rectangle(0.0, 0.0, 1.0, 1.0);
        self.cairo_context.fill().map_err(|err| err.to_string())?;

        let initial_matrix = self.cairo_context.matrix();
        self.cairo_context
            .translate(self.active_region.x, self.active_region.y);
        self.cairo_context
            .scale(self.active_region.width, self.active_region.height);

        self.cairo_context.set_source_rgba(1.0, 1.0, 1.0, 0.2);
        self.cairo_context.rectangle(0.0, 0.0, 1.0, 1.0);
        self.cairo_context.fill().map_err(|err| err.to_string())?;

        self.cairo_context.set_source_rgb(0.0, 0.0, 0.0);
        let (line_width_x, line_width_y) = self
            .cairo_context
            .device_to_user_distance(1.0, 1.0)
            .map_err(|err| err.to_string())?;
        self.cairo_context.set_line_width(line_width_x);

        self.cairo_context.move_to(0.5, 0.0);
        self.cairo_context.line_to(0.5, 1.0);
        self.cairo_context.stroke().map_err(|err| err.to_string())?;

        self.cairo_context.set_line_width(line_width_y);
        self.cairo_context.move_to(0.0, 0.5);
        self.cairo_context.line_to(1.0, 0.5);
        self.cairo_context.stroke().map_err(|err| err.to_string())?;

        self.cairo_context.set_line_width(line_width_y);
        self.cairo_context.move_to(0.0, 0.0);
        self.cairo_context.line_to(1.0, 0.0);
        self.cairo_context.stroke().map_err(|err| err.to_string())?;
        self.cairo_context.move_to(0.0, 1.0);
        self.cairo_context.line_to(1.0, 1.0);
        self.cairo_context.stroke().map_err(|err| err.to_string())?;

        self.cairo_context.set_line_width(line_width_x);
        self.cairo_context.move_to(0.0, 0.0);
        self.cairo_context.line_to(0.0, 1.0);
        self.cairo_context.stroke().map_err(|err| err.to_string())?;
        self.cairo_context.move_to(1.0, 0.0);
        self.cairo_context.line_to(1.0, 1.0);
        self.cairo_context.stroke().map_err(|err| err.to_string())?;

        self.cairo_context.set_matrix(initial_matrix);

        Ok(())
    }
    pub fn update_active_region(&mut self, rect: cairo::Rectangle) {
        let (dx, dy) = self
            .cairo_context
            .user_to_device_distance(rect.width, rect.height)
            .unwrap();
        info!("dx, dy: {}, {}", dx, dy);
        if rect.x >= 0.0
            && rect.y >= 0.0
            && rect.x < 1.0
            && rect.y < 1.0
            && dx >= 1.0
            && dy >= 1.0
        {
            self.active_region = rect;
        }
    }
    pub fn get_active_region(&self) -> cairo::Rectangle {
        self.active_region
    }
    pub fn get_buf_size(&self) -> u32 {
        (self.stride as u32) * self.height
    }
    pub fn get_format(&self) -> cairo::Format {
        self.format
    }
    pub fn get_height(&self) -> u32 {
        self.height
    }
    pub fn get_width(&self) -> u32 {
        self.width
    }
    pub fn get_stride(&self) -> i32 {
        self.stride
    }
    pub fn get_shm_fd(&self) -> i32 {
        self.temp.as_raw_fd()
    }
}
