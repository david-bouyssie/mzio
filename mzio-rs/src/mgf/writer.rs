use std::fs::File;
use std::iter::zip;
use std::io::BufWriter;
use std::io::prelude::*;
use std::path::Path;

// 3rd party imports
use anyhow::Result;

// internal imports 
use crate::mgf::spectrum::MgfSpectrum;

/// Writer for MGF files
/// Use flush() to make ensure the buffer is written completely.
pub struct MgfWriter {
    internal_writer: BufWriter<File>
}

impl MgfWriter {
    /// Creates a new Writer
    /// 
    /// # Arguments
    ///
    /// * `mgf_file_path` - Path to MGF file
    /// 
    pub fn new(mgf_file_path: &Path) -> Result<Self> {
        let mgf_file: File = File::create(mgf_file_path)?;
        Ok(Self {
            internal_writer: BufWriter::new(mgf_file)
        })
    }

    /// Writes a spectrum into the file.
    /// 
    /// # Arguments
    ///
    /// * `spectrum` - Spectrum
    /// 
    pub fn write_spectrum(&mut self, spectrum: &MgfSpectrum) -> Result<usize> {
        let spec_header = &spectrum.header;

        let mut written_bytes: usize = 0;

        written_bytes += self._write_str("BEGIN IONS\n")?;
        written_bytes += self._write_string(format!("TITLE={}\n", spec_header.get_title()))?;
        written_bytes += self._write_string(format!("PEPMASS={}", spec_header.get_precursor_mz()))?;

        if let Some(retention_time) = spec_header.get_retention_time() {
            written_bytes += self._write_string(format!("\nRTINSECONDS={}", retention_time))?;
        }
        if let Some(charge) = spec_header.get_precursor_charge() {
            let charge_sign = if charge < 0 { '-'} else { '+' };
            written_bytes += self._write_string(format!("\nCHARGE={}{}", charge, charge_sign))?;
        }
        for (mz, intensity) in zip(spectrum.get_mz_list(), spectrum.get_intensity_list()) {
            written_bytes += self._write_string(format!("\n{mz} {intensity}"))?;
        }
        written_bytes += self._write_str("\nEND IONS\n")?;

        Ok(written_bytes)
    }

    #[inline(always)]
    fn _write_str(&mut self, str: &str) -> Result<usize> {
        Ok(self.internal_writer.write(str.as_bytes())?)
    }

    #[inline(always)]
    fn _write_string(&mut self, string: String) -> Result<usize> {
        Ok(self.internal_writer.write(string.as_bytes())?)
    }

    /// Writes multiple spectra to file.
    /// 
    /// # Arguments
    ///
    /// * `spectra` - Iterator of spectra
    /// 
    pub fn write_all<'b, I>(&mut self, spectra: I) -> Result<usize>
    where
        I: Iterator<Item = &'b MgfSpectrum>,
    {
        let mut written_bytes: usize = 0;
        for spectrum in spectra {
            written_bytes += self.write_spectrum(spectrum)?;
        }
        return Ok(written_bytes);
    }

    /// Flushes the buffer
    /// 
    pub fn flush(&mut self) -> Result<()> {
        self.internal_writer.flush()?;
        Ok(())
    }
}