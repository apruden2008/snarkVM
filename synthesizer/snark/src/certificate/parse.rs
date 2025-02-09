// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

use super::*;

static PROOF_PREFIX: &str = "certificate";

impl<N: Network> Parser for Certificate<N> {
    /// Parses a string into an certificate.
    #[inline]
    fn parse(string: &str) -> ParserResult<Self> {
        // Prepare a parser for the Aleo certificate.
        let parse_certificate = recognize(pair(
            pair(tag(PROOF_PREFIX), tag("1")),
            many1(terminated(one_of("qpzry9x8gf2tvdw0s3jn54khce6mua7l"), many0(char('_')))),
        ));

        // Parse the certificate from the string.
        map_res(parse_certificate, |certificate: &str| -> Result<_, Error> {
            Self::from_str(&certificate.replace('_', ""))
        })(string)
    }
}

impl<N: Network> FromStr for Certificate<N> {
    type Err = Error;

    /// Reads in the certificate string.
    fn from_str(certificate: &str) -> Result<Self, Self::Err> {
        // Decode the certificate string from bech32m.
        let (hrp, data, variant) = bech32::decode(certificate)?;
        if hrp != PROOF_PREFIX {
            bail!("Failed to decode certificate: '{hrp}' is an invalid prefix")
        } else if data.is_empty() {
            bail!("Failed to decode certificate: data field is empty")
        } else if variant != bech32::Variant::Bech32m {
            bail!("Found an certificate that is not bech32m encoded: {certificate}");
        }
        // Decode the certificate data from u5 to u8, and into the certificate.
        Ok(Self::read_le(&Vec::from_base32(&data)?[..])?)
    }
}

impl<N: Network> Debug for Certificate<N> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl<N: Network> Display for Certificate<N> {
    /// Writes the certificate as a bech32m string.
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Convert the certificate to bytes.
        let bytes = self.to_bytes_le().map_err(|_| fmt::Error)?;
        // Encode the bytes into bech32m.
        let string =
            bech32::encode(PROOF_PREFIX, bytes.to_base32(), bech32::Variant::Bech32m).map_err(|_| fmt::Error)?;
        // Output the string.
        Display::fmt(&string, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use console::network::Testnet3;

    type CurrentNetwork = Testnet3;

    #[test]
    fn test_parse() -> Result<()> {
        // Ensure type and empty value fails.
        assert!(Certificate::<CurrentNetwork>::parse(&format!("{PROOF_PREFIX}1")).is_err());
        assert!(Certificate::<CurrentNetwork>::parse("").is_err());

        // Sample the certificate.
        let certificate = crate::test_helpers::sample_certificate();

        // Check the certificate parsing.
        let expected = format!("{certificate}");
        let (remainder, candidate) = Certificate::<CurrentNetwork>::parse(&expected).unwrap();
        assert_eq!(format!("{expected}"), candidate.to_string());
        assert_eq!(PROOF_PREFIX, candidate.to_string().split('1').next().unwrap());
        assert_eq!("", remainder);
        Ok(())
    }

    #[test]
    fn test_string() -> Result<()> {
        // Sample the certificate.
        let expected = crate::test_helpers::sample_certificate();

        // Check the string representation.
        let candidate = format!("{expected}");
        assert_eq!(expected, Certificate::from_str(&candidate)?);
        assert_eq!(PROOF_PREFIX, candidate.split('1').next().unwrap());

        Ok(())
    }

    #[test]
    fn test_display() -> Result<()> {
        // Sample the certificate.
        let expected = crate::test_helpers::sample_certificate();

        let candidate = expected.to_string();
        assert_eq!(format!("{expected}"), candidate);
        assert_eq!(PROOF_PREFIX, candidate.split('1').next().unwrap());

        let candidate_recovered = Certificate::<CurrentNetwork>::from_str(&candidate)?;
        assert_eq!(expected, candidate_recovered);

        Ok(())
    }
}
