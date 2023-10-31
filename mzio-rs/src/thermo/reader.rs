
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
