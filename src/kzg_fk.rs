use fastcrypto::error::{FastCryptoError, FastCryptoResult};
use fastcrypto::groups::bls12381::{G1Element, G2Element, Scalar};
use fastcrypto::groups::{GroupElement, MultiScalarMul, Pairing, Scalar as OtherScalar};
use rand::thread_rng;
use std::ops::Mul;

use crate::fft::{BLS12381Domain, FFTDomain};
use crate::KZG;


pub struct KZGFK {
    domain: BLS12381Domain,
    tau_powers_g1: Vec<G1Element>,
    tau_powers_g2: Vec<G2Element>,
}

impl KZGFK {
    pub fn new(n: usize) -> FastCryptoResult<Self> {
        let domain = BLS12381Domain::new(n)?;

        // Generate tau using a random scalar
        let tau = Scalar::rand(&mut thread_rng());

        // Compute g^tau^i for i = 0 to n-1 in G1
        let tau_powers_g1: Vec<G1Element> = itertools::iterate(G1Element::generator(), |g| g * tau)
            .take(n)
            .collect();

        // Compute g^tau^i for i = 0 to n-1 in G2
        let tau_powers_g2: Vec<G2Element> = itertools::iterate(G2Element::generator(), |g| g * tau)
            .take(n)
            .collect();

        Ok(Self {
            domain,
            tau_powers_g1,
            tau_powers_g2,
        })
    }
}

impl KZG for KZGFK {
    // Uses the BLS12-381 construction
    type G = G1Element;

    fn commit(&self, v: &[Scalar]) -> G1Element {
        let poly = self.domain.ifft(&v);
        G1Element::multi_scalar_mul(&poly, &self.tau_powers_g1).unwrap()
    }

    fn open(&self, v: &[Scalar], index: usize) -> G1Element {
        let mut poly = self.domain.ifft(&v);
        let mut quotient_coeffs: Vec<Scalar> = vec![Scalar::zero(); poly.len()-1];

        quotient_coeffs[poly.len() - 2] = poly[poly.len() - 1];

        for j in (0..poly.len() - 2).rev() {
            quotient_coeffs[j] = poly[j + 1] + quotient_coeffs[j + 1] * self.domain.element(index);
        }

        G1Element::multi_scalar_mul(&quotient_coeffs, &self.tau_powers_g1[..quotient_coeffs.len()]).unwrap()
        // let mut open_value = G1Element::zero();
        // for (i, coeff) in quotient_coeffs.iter().enumerate() {
        //     open_value += self.tau_powers_g1[i].mul(*coeff);
        // }
        // open_value
    }

    fn verify(
        &self,
        index: usize,
        v_i: &Scalar,
        commitment: &G1Element,
        open_i: &G1Element,
    ) -> bool {
        let lhs = *commitment - self.tau_powers_g1[0] * v_i;

        let rhs = self.tau_powers_g2[1] - self.tau_powers_g2[0] * self.domain.element(index);

        // Perform the pairing check e(lhs, g) == e(open_i, rhs)
        let lhs_pairing = lhs.pairing(&self.tau_powers_g2[0]);
        let rhs_pairing = open_i.pairing(&rhs);

        lhs_pairing == rhs_pairing
    }

    fn update(
        &self,
        commitment: &mut G1Element,
        index: usize,
        old_v_i: &Scalar, 
        new_v_i: &Scalar
    ) -> G1Element {
        *commitment
    }

    fn update_open_i(&self, open: &mut G1Element, index: usize, old_v_i: &Scalar, new_v_i: &Scalar) -> G1Element{
        *open
    }
}

#[cfg(test)]
mod tests {
    use fastcrypto::groups::bls12381::Scalar;
    use rand::Rng;

    use super::*;

    #[test]
    fn test_kzg_commit_open_verify() {
        let mut rng = rand::thread_rng();

        // Create a new KZGFK struct
        let n = 8;
        let kzg = KZGFK::new(n).unwrap();

        // Create a random vector v
        let v: Vec<Scalar> = (0..n).map(|_| OtherScalar::rand(&mut rng)).collect();

        println!("{:?}", v);

        // Create a commitment
        let commitment = kzg.commit(&v);

        // Pick a random index to open
        let index = rng.gen_range(0..n);

        // Create an opening
        let open_value = kzg.open(&v, index);

        // Verify the opening
        let is_valid = kzg.verify(index, &v[index], &commitment, &open_value);

        // Assert that the verification passes
        assert!(is_valid, "Verification of the opening should succeed.");
    }
}
