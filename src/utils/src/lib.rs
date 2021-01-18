pub mod zlib {
    use flate2::{
        write::{ZlibDecoder, ZlibEncoder},
        Compression,
    };

    use myutil::{err::*, *};
    use std::io::Write;

    pub fn encode(data: &[u8]) -> Result<Vec<u8>> {
        let mut encoder = ZlibEncoder::new(vec![], Compression::default());
        encoder.write_all(data).c(d!())?;
        encoder.finish().c(d!())
    }

    pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
        let mut decoder = ZlibDecoder::new(vec![]);
        decoder.write_all(data).c(d!()).and_then(|_| decoder.finish().c(d!()))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rand::random;

        #[test]
        fn it_works() {
            (0..(10 + random::<u8>() % 20))
                .map(|i| (0..i).map(|_| random::<u8>()).collect::<Vec<_>>())
                .for_each(|sample| {
                    assert_eq!(sample, pnk!(decode(&pnk!(encode(&sample)))));
                });
        }
    }
}
