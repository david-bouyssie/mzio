
// std imports
use std::path::Path;
use std::rc::Rc;
use std::cell::RefCell;

// 3rd party imports
use anyhow::*;
use fallible_iterator::FallibleIterator;
use thermostreaming::*;

// internal imports

/// Reader for Thermo raw file
pub struct ThermoReader {
    thermo_streamer_ref: Rc<RefCell<RawFileStreamer>>,
}

impl ThermoReader {
    /// Creates a new Reader
    ///
    /// # Arguments
    ///
    /// * `raw_file_path` - Path to Thermo raw file
    ///
    pub fn new(raw_file_path: &Path, raw_file_parser_path: &Path) -> Result<Self> {

        // Configure in a subscope to release the lock guard
        {
            let mut e4k = MONO_EMBEDDINATOR.lock().unwrap();
            if !e4k.is_configured() {
                let parser_dir_str = raw_file_parser_path.to_string_lossy().to_string();
                e4k.configure(parser_dir_str.as_str())?;
            }

            e4k.check_availability()?;
        }

        let raw_file_path_str = raw_file_path.to_string_lossy().to_string();

        let streamer = RawFileStreamer::new(raw_file_path_str.as_str())?;
        let shared_streamer = Rc::new(RefCell::new(streamer));

        Ok(Self {
            thermo_streamer_ref: shared_streamer,
        })
    }

    pub fn get_raw_file_path(&self) -> String {
        self.thermo_streamer_ref.borrow().get_raw_file_path().to_string()
    }

    pub fn get_first_scan_number(&self) -> u32 {
        self.thermo_streamer_ref.borrow().get_first_scan_number()
    }

    pub fn get_last_scan_number(&self) -> u32 {
        self.thermo_streamer_ref.borrow().get_last_scan_number()
    }

    fn read_spectrum(streamer: &Rc<RefCell<RawFileStreamer>>, number: u32) -> Result<MzMLSpectrum> {
        streamer.borrow().get_spectrum(number)
        /*let mzml_spectrum = streamer.borrow().get_spectrum(number)?;
        let (metadata_opt,data_opt) = (mzml_spectrum.metadata, mzml_spectrum.data);
        let data = data_opt.unwrap();
        let intensities = data.intensity_list.iter().map(|&i| i as f32).collect();

        Ok(MzMLSpectrum {
            metadata: metadata_opt.unwrap(),
            data: mzcore::ms::spectrum::SpectrumData {
                mz_list: data.mz_list,
                intensity_list: intensities
            }
        })*/
    }

    pub fn get_spectrum(&self, number: u32) -> Result<MzMLSpectrum> {
        self.thermo_streamer_ref.borrow().get_spectrum(number)
    }

    pub fn iter(&self) -> ThermoSpectrumIter {
        ThermoSpectrumIter::new(Rc::clone(&self.thermo_streamer_ref))
    }

    pub fn ms2_iter(&self) -> impl FallibleIterator<Item = MzMLSpectrum, Error = Error> {
        let x= ThermoSpectrumIter::new(Rc::clone(&self.thermo_streamer_ref)).filter_map(|s| {
            if s.get_ms_level() == 2 {
                Ok(Some(s))
            } else {
                Ok(None)
            }
        });

        x
    }

    pub fn process_spectra_in_parallel<F>(&self, mut on_each_spectrum: F, queue_size: usize) -> Result<()>
    where
        F: FnMut(Result<MzMLSpectrum>) -> Result<()> + Send + Sync,
     {
         self.thermo_streamer_ref.borrow().process_spectra_in_parallel(|spectrum| {
             on_each_spectrum(Ok(spectrum?))
         }, queue_size)
     }
}

pub struct ThermoSpectrumIter {
    thermo_streamer_ref: Rc<RefCell<RawFileStreamer>>,
    current_spectrum_number: u32,
    last_spectrum_number: u32,
}

impl ThermoSpectrumIter {
    fn new(reader: Rc<RefCell<RawFileStreamer>>) -> ThermoSpectrumIter {
        let thermo_streamer_ref = Rc::clone(&reader);
        let thermo_streamer = thermo_streamer_ref.borrow();
        let first_scan_number = thermo_streamer.get_first_scan_number();
        let last_scan_number = thermo_streamer.get_last_scan_number();

        Self {
            thermo_streamer_ref: reader,
            current_spectrum_number: first_scan_number,
            last_spectrum_number: last_scan_number,
        }
    }
}

impl FallibleIterator for ThermoSpectrumIter {
    type Item = MzMLSpectrum;
    type Error = anyhow::Error;

    fn next(&mut self) -> Result<Option<Self::Item>> {

        let cur_spec_num = self.current_spectrum_number;
        if cur_spec_num > self.last_spectrum_number {
            return Ok(None);
        }

        self.current_spectrum_number += 1;

        //let spectrum = ThermoReader::read_spectrum(&self.thermo_streamer_ref, cur_spec_num)?;
        Ok(Some(self.thermo_streamer_ref.borrow().get_spectrum(cur_spec_num)?))
    }
}

/*
use std::sync::{mpsc, Arc, Mutex, RwLock};
use std::thread;

pub struct ThermoReader {
    thermo_streamer_ref: Arc<Mutex<RawFileStreamer>>,
}

impl ThermoReader {
    /// Creates a new Reader
    ///
    /// # Arguments
    ///
    /// * `raw_file_path` - Path to Thermo raw file
    ///
    pub fn new(raw_file_path: &Path, raw_file_parser_path: &Path) -> Result<Self> {

        // Configure in a subscope to release the lock guard
        {
            let mut e4k = MONO_EMBEDDINATOR.lock().unwrap();
            if !e4k.is_configured() {
                let parser_dir_str = raw_file_parser_path.to_string_lossy().to_string();
                e4k.configure(parser_dir_str.as_str())?;
            }

            e4k.check_availability()?;
        }

        let raw_file_path_str = raw_file_path.to_string_lossy().to_string();

        let streamer = RawFileStreamer::new(raw_file_path_str.as_str())?;
        let shared_streamer = Arc::new(Mutex::new(streamer));

        Ok(Self {
            thermo_streamer_ref: shared_streamer,
        })
    }

    pub fn get_raw_file_path(&self) -> String {
        self.thermo_streamer_ref.lock().unwrap().get_raw_file_path().to_string()
    }

    pub fn get_first_scan_number(&self) -> u32 {
        self.thermo_streamer_ref.lock().unwrap().get_first_scan_number()
    }

    pub fn get_last_scan_number(&self) -> u32 {
        self.thermo_streamer_ref.lock().unwrap().get_last_scan_number()
    }

    fn read_spectrum(streamer: &Arc<Mutex<RawFileStreamer>>, number: u32) -> Result<MzMLSpectrum> {
        let (metadata_opt,data_opt) = streamer.lock().unwrap().get_spectrum(number, true, true)?;
        let data = data_opt.unwrap();
        let intensities = data.intensity_list.iter().map(|&i| i as f32).collect();

        Ok(MzMLSpectrum {
            metadata: metadata_opt.unwrap(),
            data: mzcore::ms::spectrum::SpectrumData {
                mz_list: data.mz_list,
                intensity_list: intensities
            }
        })
    }

    /*fn read_spectrum(streamer: &Rc<RefCell<RawFileStreamer>>, number: u32) -> Result<MzMLSpectrum> {
        let (metadata_opt,data_opt) = streamer.borrow().get_spectrum(number, true, true)?;
        let data = data_opt.unwrap();
        let intensities = data.intensity_list.iter().map(|&i| i as f32).collect();

        Ok(MzMLSpectrum {
            metadata: metadata_opt.unwrap(),
            data: mzcore::ms::spectrum::SpectrumData {
                mz_list: data.mz_list,
                intensity_list: intensities
            }
        })
    }*/

    pub fn get_spectrum(&self, number: u32) -> Result<MzMLSpectrum> {
        ThermoReader::read_spectrum(&self.thermo_streamer_ref, number)
    }

    /*pub fn iter(&self) -> fallible_iterator::Iterator<ThermoSpectrumIter> {
        ThermoSpectrumIter::new(&self.thermo_streamer).iterator().into_iter()
    }*/

    /*pub fn iter(&self) -> ThermoSpectrumIter {
        ThermoSpectrumIter::new(&self.thermo_streamer)
    }*/

    pub fn iter(&self) -> ThermoSpectrumIter {
        ThermoSpectrumIter::new(&self.thermo_streamer_ref)
    }

    // Filter<ThermoSpectrumIter, fn(&MzMLSpectrum) -> std::result::Result<bool, dyn std::error::Error>>
    pub fn ms2_iter(&self) -> impl FallibleIterator<Item = MzMLSpectrum, Error = Error> {
        let x= ThermoSpectrumIter::new(&self.thermo_streamer_ref).filter_map(|s| {
            if s.get_ms_level() == 2 {
                Ok(Some(s))
            } else {
                Ok(None)
            }
        });

        x
    }

}

pub struct ThermoSpectrumIter {
    thermo_streamer_ref: Arc<Mutex<RawFileStreamer>>,
    current_spectrum_number: u32,
    last_spectrum_number: u32,
    receiver: Option<mpsc::Receiver<MzMLSpectrum>>,
}

impl ThermoSpectrumIter {
    fn new(reader: &Arc<Mutex<RawFileStreamer>>) -> ThermoSpectrumIter {
        let thermo_streamer_ref = Arc::clone(&reader);
        let first_scan_number = thermo_streamer.get_first_scan_number();
        let last_scan_number = thermo_streamer.get_last_scan_number();

        ThermoSpectrumIter {
            thermo_streamer_ref: thermo_streamer_ref,
            current_spectrum_number: first_scan_number,
            last_spectrum_number: last_scan_number,
            receiver: None,
        }
    }

    // Function to spawn a thread for processing the spectrum
    fn spawn_spectrum_processing_thread(&mut self, number: u32) {
         let thermo_streamer_ref = Arc::clone(&self.thermo_streamer_ref);

        let (sender, receiver) = mpsc::sync_channel(1);
        self.receiver = Some(receiver);

        thread::spawn(move || {
            /*let spectrum = ThermoReader::read_spectrum(&thermo_streamer_ref, number);
            match spectrum {
                Ok(spectrum) => {
                    if let Err(_) = sender.send(spectrum) {
                        eprintln!("Failed to send spectrum through channel");
                    }
                }
                Err(err) => {
                    eprintln!("Error reading spectrum: {:?}", err);
                }
            }*/

             unsafe {
                // FIXME: no unwrap
                let spectrum = {
                    //let mut thermo_streamer = thermo_streamer_ref.lock().unwrap();
                    ThermoReader::read_spectrum(&thermo_streamer_ref, number).unwrap()
                };

                sender.send(spectrum);
             }
        });
    }
}

impl FallibleIterator for ThermoSpectrumIter {
    type Item = MzMLSpectrum;
    type Error = anyhow::Error;

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        let cur_spec_num = self.current_spectrum_number;
        if cur_spec_num > self.last_spectrum_number {
            return Ok(None);
        }

        self.spawn_spectrum_processing_thread(cur_spec_num);
        /*let spectrum = match &self.sender {
            Some(sender) => sender.recv().map_err(|e| anyhow::Error::msg(e.to_string()))?,
            None => return Err(anyhow::Error::msg("Sender not initialized")),
        };*/

        /*let spectrum = match &self.receiver {
            Some(receiver) => match receiver.recv() {
                anyhow::Ok(spectrum) => spectrum,
                Err(_) => return Err(anyhow::Error::msg("Failed to receive spectrum from channel")),
            },
            None => return Err(anyhow::Error::msg("Receiver not initialized")),
        };*/

        let spectrum = match &self.receiver {
            Some(receiver) => receiver.recv().map_err(|_| anyhow::Error::msg("Failed to receive spectrum from channel"))?,
            None => return Err(anyhow::Error::msg("Receiver not initialized")),
        };

        self.current_spectrum_number += 1;

        Ok(Some(spectrum))
    }
}*/