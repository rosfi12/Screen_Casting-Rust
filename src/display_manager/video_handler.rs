use std::fs::{self, File};
use std::io::{Write, Seek, SeekFrom};
use byteorder::{LittleEndian, WriteBytesExt};

pub struct VideoWriter {
    frames: Vec<Vec<u8>>,
    frame_count: usize,
    file_path: String,
    width: u32,
    height: u32,
    fps: u32,
}

impl VideoWriter {
    pub fn new(frame_limit: usize, file_path: String) -> Self {
        fs::create_dir_all("recordings").unwrap();
        Self {
            frames: Vec::with_capacity(frame_limit),
            frame_count: 0,
            file_path: format!("recordings/{}", file_path),
            width: 1920,
            height: 1080,
            fps: 30,
        }
    }

    pub fn add_frame(&mut self, frame: Vec<u8>) {
        self.frames.push(frame);
        self.frame_count += 1;
    }

    pub fn write_to_file(&mut self) {
        if self.frames.is_empty() {
            return;
        }

        let mut file = File::create(&self.file_path).unwrap();
        
        // Write AVI header
        file.write_all(b"RIFF").unwrap();
        let header_pos = file.stream_position().unwrap();
        file.write_u32::<LittleEndian>(0).unwrap(); // File size placeholder
        file.write_all(b"AVI ").unwrap();
        
        // Write main header
        let frames_len: u32 = self.frames.len() as u32;
        file.write_all(b"LIST").unwrap();
        file.write_u32::<LittleEndian>(192).unwrap();
        file.write_all(b"hdrl").unwrap();
        
        // Write frame data
        for frame in &self.frames {
            file.write_all(b"00dc").unwrap();
            file.write_u32::<LittleEndian>(frame.len() as u32).unwrap();
            file.write_all(frame).unwrap();
        }

        // Update file size
        let end_pos = file.stream_position().unwrap();
        file.seek(SeekFrom::Start(header_pos)).unwrap();
        file.write_u32::<LittleEndian>((end_pos - 8) as u32).unwrap();

        self.frames.clear();
        self.frame_count = 0;
    }
}