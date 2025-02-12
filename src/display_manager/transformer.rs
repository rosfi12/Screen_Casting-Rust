pub mod encoder {
    use openh264::{encoder::Encoder, formats::{RgbSliceU8, YUVBuffer}};
    use std::time::Instant;
    use image::{self, ImageBuffer};

    // take in input an image in RGB and convert it in YUV because encoder H.264 works well on this format

    pub fn encode<'a>(rgb_image: &ImageBuffer<image::Rgb<u8>, Vec<u8>>) -> (u32, u32, Vec<u8>, std::time::Duration) {
        
        let (width_image, height_image) = rgb_image.dimensions();
       
        let yuv_buffer = YUVBuffer::from_rgb_source(RgbSliceU8::new(&rgb_image
        , (width_image as usize, height_image as usize)));
        
        let mut encoder = Encoder::new().unwrap();

        let start_encode = Instant::now();
        let encoded_frames = encoder.encode(&yuv_buffer).expect("Not encode frame").to_vec();
        let encode_duration = start_encode.elapsed();
    
        (width_image, height_image, encoded_frames, encode_duration)
    }
}
