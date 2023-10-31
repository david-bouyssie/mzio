/// Module for dealing with MGF files

pub mod reader;
pub mod prelude;

pub use prelude::*;

#[cfg(test)]
mod test {
    use super::*;

    use std::path::Path;
    use mzcore::ms::spectrum::SpectrumData;
    use crate::mgf::*;
    use crate::mgf::spectrum::MgfSpectrumHeader;

    const RAW_FILE_PARSER_PATH_STR: &'static str =
        if cfg!(debug_assertions) {
            "../target/debug/rawfileparser"
        } else {
            "../target/release/rawfileparser"
        };

    const RAW_FILE_PATH_STR: &'static str = "../test_files/thermo/small.RAW";
    const TEMP_MGF_PATH_STR: &'static str = "../test_files/mgf/Velos005137.mgf.tmp";
    const EXPECTED_NUM_SPECTRA: usize = 48;
    const EXPECTED_NUM_MS2_SPECTRA: usize = 34;

    // WARNING: it is not possible to run multiple tests because rust unit tests are started in different threads
    // So all the tests have to be performed within the same function
    #[test]
    fn test_thermo_reader() {
        let rawfileparser_path = Path::new(RAW_FILE_PARSER_PATH_STR);
        let raw_file_path = Path::new(RAW_FILE_PATH_STR);
        let tmp_mgf_file_path = Path::new(TEMP_MGF_PATH_STR);

        let thermo_reader = ThermoReader::new(raw_file_path, rawfileparser_path).unwrap();

        // Call iter() twice should work
        let entries1: Vec<MzMLSpectrum> = thermo_reader.iter().collect().unwrap();
        let entries2: Vec<MzMLSpectrum> = thermo_reader.iter().collect().unwrap();
        assert_eq!(entries1.len(), EXPECTED_NUM_SPECTRA);
        assert_eq!(entries2.len(), EXPECTED_NUM_SPECTRA);

        // Record the starting time
        let start = std::time::Instant::now();

        let mut mgf_writer = MgfWriter::with_capacity(4 * 1024 * 1024, tmp_mgf_file_path).unwrap();

        thermo_reader.process_spectra_in_parallel(|spectrum_res|{

            let spectrum= spectrum_res?;
            if spectrum.get_ms_level() == 2 {
                let ms2_spec = spectrum;

                let (prec_mz_opt, perc_charge_opt) = ms2_spec.get_precursor_mz_and_charge();
                let smd = &ms2_spec.metadata;
                let scan_idx = smd.index.parse::<u32>().unwrap();

                let mgf_spec = MgfSpectrum {
                    header: MgfSpectrumHeader {
                        title: smd.id.clone(),
                        precursor_mz: prec_mz_opt.unwrap_or(0.0),
                        precursor_charge: perc_charge_opt,
                        precursor_mass: None,
                        retention_time: ms2_spec.get_first_scan_start_time().clone().map(|rt_minutes| rt_minutes * 60.0),
                        scan_number: Some(scan_idx)
                    },
                    data: SpectrumData {
                        mz_list: ms2_spec.data.mz_list,
                        intensity_list: ms2_spec.data.intensity_list.iter().map(|&i| i as f32).collect()
                    },
                };

                mgf_writer.write_spectrum(&mgf_spec, true).unwrap();
            }

            Ok(())
        }, 10).unwrap();

        mgf_writer.flush().unwrap();

        // Record the ending time
        let duration = start.elapsed();

        println!("Has converted RAW file to MGF file in: {:?}", duration);

        // Reload the produced MGF file to test the number of written spectra
        let mgf_reader = MgfReader::new(
            tmp_mgf_file_path,
            1024
        ).unwrap();

        let mgf_entries: Vec<MgfSpectrum> = mgf_reader.collect().unwrap();
        assert_eq!(mgf_entries.len(), EXPECTED_NUM_MS2_SPECTRA);

        std::fs::remove_file(tmp_mgf_file_path).unwrap();

        ()
    }

}
