use rodio::{cpal::{self, FromSample, Sample, traits::{HostTrait, StreamTrait}}, DeviceTrait, Source};
use std::io::BufReader;
use std::thread;
use std::time::Duration;
use rodio::*;
use std::sync::{Arc, Mutex};
use std::io::BufWriter;
use std::fs::File;
use hound::*;

// https://github.com/RustAudio/cpal/blob/master/examples/record_wav.rs




type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

fn main() {


    get_input_stream();
}


fn sample_format(format: cpal::SampleFormat) -> hound::SampleFormat {
    if format.is_float() {
        hound::SampleFormat::Float
    } else {
        hound::SampleFormat::Int
    }
}

fn wav_spec_from_config(config: &cpal::SupportedStreamConfig) -> hound::WavSpec {
    hound::WavSpec {
        channels: config.channels() as _,
        sample_rate: config.sample_rate().0 as _,
        bits_per_sample: (config.sample_format().sample_size() * 8) as _,
        sample_format: sample_format(config.sample_format()),
    }
}


fn write_input_data<T, U>(input: &[T], writer: &WavWriterHandle)
where
    T: Sample,
    U: Sample + hound::Sample + FromSample<T>,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for &sample in input.iter() {
                let sample: U = U::from_sample(sample);
                writer.write_sample(sample).ok();
            }
        }
    }
}


fn get_input_stream() {
    let host = cpal::default_host();
    let device = host.default_input_device().unwrap();
    let dev:rodio::Device = device.into();
    // let dev_name:String = dev.name().unwrap();
    println!(
        "  {} - Supported input configs:",
        dev.name()
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("<unknown>"),
    );

    let config = dev
        .default_input_config()
        .expect("Failed to get default input config");
println!("Default input config: {:?}", config);


    const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded.wav");
    let spec = wav_spec_from_config(&config);
    let writer = hound::WavWriter::create(PATH, spec).unwrap();
    let writer = Arc::new(Mutex::new(Some(writer)));
    let writer_2 = writer.clone();

    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };

    let stream = match config.sample_format() {
        cpal::SampleFormat::I8 => dev.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i8, i8>(data, &writer_2),
            err_fn,
            None,
        ).unwrap(),
        cpal::SampleFormat::I16 => dev.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i16, i16>(data, &writer_2),
            err_fn,
            None,
        ).unwrap(),
        cpal::SampleFormat::I32 => dev.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i32, i32>(data, &writer_2),
            err_fn,
            None,
        ).unwrap(),
        cpal::SampleFormat::F32 => dev.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<f32, f32>(data, &writer_2),
            err_fn,
            None,
        ).unwrap(),
        sample_format => {
            return ()
        }
    };


    stream.play().unwrap();
    thread::sleep(Duration::from_secs(10));
    stream.pause().unwrap();

    std::thread::sleep(std::time::Duration::from_secs(3));
    drop(stream);
    writer.lock().unwrap().take().unwrap().finalize().unwrap();
    println!("Recording {} complete!", PATH);



 }


fn get_output_stream(device_name:&str) -> (OutputStream,OutputStreamHandle) {
    let host = cpal::default_host();
    let devices = host.output_devices().unwrap();
    let ( mut _stream, mut stream_handle) = OutputStream::try_default().unwrap();
    for device in devices{ 
       let dev:rodio::Device = device.into();
       let dev_name:String=dev.name().unwrap();
       if dev_name==device_name {
          println!("Device found: {}", dev_name);
          ( _stream, stream_handle) = OutputStream::try_from_device(&dev).unwrap();
       }
    }

    
    return (_stream,stream_handle);
 }

fn reverb() {
    let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
    let sink = rodio::Sink::try_new(&handle).unwrap();

    let file = std::fs::File::open("assets/music.ogg").unwrap();
    let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
    let with_reverb = source.buffered().reverb(Duration::from_millis(40), 0.7);
    sink.append(with_reverb);

    sink.sleep_until_end();
}

fn list_devices() {
    let host = cpal::default_host();
    let devices = host.output_devices();
    
    for val in devices.unwrap() {
        let dev:rodio::Device = val.into();
        let dev_name:String = dev.name().unwrap();
        println!("Got: {}", dev_name);
    }
}


fn spatial() {
    let iter_duration = Duration::from_secs(5);
    let iter_distance = 5.;

    let refresh_duration = Duration::from_millis(10);

    let num_steps = iter_duration.as_secs_f32() / refresh_duration.as_secs_f32();
    let step_distance = iter_distance / num_steps;
    let num_steps = num_steps as u32;

    let repeats = 5;

    let total_duration = iter_duration * 2 * repeats;

    let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
    let mut positions = ([0., 0., 0.], [-1., 0., 0.], [1., 0., 0.]);
    let sink = rodio::SpatialSink::try_new(&handle, positions.0, positions.1, positions.2).unwrap();

    let file = std::fs::File::open("assets/music.ogg").unwrap();
    let source = rodio::Decoder::new(BufReader::new(file))
        .unwrap()
        .repeat_infinite()
        .take_duration(total_duration);
    sink.append(source);
    // A sound emitter playing the music starting at the centre gradually moves to the right
    // until it stops and begins traveling to the left, it will eventually pass through the
    // listener again and go to the far left.
    // This is repeated 5 times.
    for _ in 0..repeats {
        for _ in 0..num_steps {
            thread::sleep(refresh_duration);
            positions.0[0] += step_distance;
            sink.set_emitter_position(positions.0);
        }
        for _ in 0..(num_steps * 2) {
            thread::sleep(refresh_duration);
            positions.0[0] -= step_distance;
            sink.set_emitter_position(positions.0);
        }
        for _ in 0..num_steps {
            thread::sleep(refresh_duration);
            positions.0[0] += step_distance;
            sink.set_emitter_position(positions.0);
        }
    }
    sink.sleep_until_end();
}