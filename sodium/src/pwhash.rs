//! This module provides access to libsodium

use super::{check_init, secbuf::SecBuf};
use crate::error::SodiumError;

pub const OPSLIMIT_INTERACTIVE: u64 = rust_sodium_sys::crypto_pwhash_OPSLIMIT_INTERACTIVE as u64;
pub const MEMLIMIT_INTERACTIVE: usize =
    rust_sodium_sys::crypto_pwhash_MEMLIMIT_INTERACTIVE as usize;
pub const OPSLIMIT_MODERATE: u64 = rust_sodium_sys::crypto_pwhash_OPSLIMIT_MODERATE as u64;
pub const MEMLIMIT_MODERATE: usize = rust_sodium_sys::crypto_pwhash_MEMLIMIT_MODERATE as usize;
pub const OPSLIMIT_SENSITIVE: u64 = rust_sodium_sys::crypto_pwhash_OPSLIMIT_SENSITIVE as u64;
pub const MEMLIMIT_SENSITIVE: usize = rust_sodium_sys::crypto_pwhash_MEMLIMIT_SENSITIVE as usize;

pub const ALG_ARGON2I13: i8 = rust_sodium_sys::crypto_pwhash_ALG_ARGON2I13 as i8;
pub const ALG_ARGON2ID13: i8 = rust_sodium_sys::crypto_pwhash_ALG_ARGON2ID13 as i8;

pub const HASHBYTES: usize = 32 as usize;
pub const SALTBYTES: usize = rust_sodium_sys::crypto_pwhash_SALTBYTES as usize;

/// Calculate a password hash
///
/// @param {SecBuf} password - the password to hash
///
/// @param {u64} opslimit - operation scaling for hashing algorithm
///
/// @param {usize} memlimit - memory scaling for hashing algorithm
///
/// @param {i8} algorithm - which hashing algorithm
///
/// @param {SecBuf} salt - predefined salt (randomized it if you dont want to generate it )
///
/// @param {SecBuf} hash - the hash generated
pub fn hash(
    password: &mut SecBuf,
    ops_limit: u64,
    mem_limit: usize,
    alg: i8,
    salt: &mut SecBuf,
    hash: &mut SecBuf,
) -> Result<(), SodiumError> {
    check_init();
    let salt = salt.read_lock();
    let password = password.read_lock();
    let mut hash = hash.write_lock();
    let hash_len = hash.len() as libc::c_ulonglong;
    let pw_len = password.len() as libc::c_ulonglong;
    let res = unsafe {
        rust_sodium_sys::crypto_pwhash(
            raw_ptr_char!(hash),
            hash_len,
            raw_ptr_ichar_immut!(password),
            pw_len,
            raw_ptr_char_immut!(salt),
            ops_limit as libc::c_ulonglong,
            mem_limit,
            alg as libc::c_int,
        )
    };
    match res {
        0 => Ok(()),
        -1 => Err(SodiumError::OutOfMemory),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_generate_with_random_salt() {
        let mut password = SecBuf::with_secure(HASHBYTES);
        let mut pw1_hash = SecBuf::with_secure(HASHBYTES);
        let mut random_salt = SecBuf::with_insecure(SALTBYTES);
        password.randomize();
        random_salt.randomize();
        hash(
            &mut password,
            OPSLIMIT_SENSITIVE,
            MEMLIMIT_SENSITIVE,
            ALG_ARGON2ID13,
            &mut random_salt,
            &mut pw1_hash,
        )
        .unwrap();
        assert_eq!(HASHBYTES, password.len());
    }
    #[test]
    fn it_should_generate_with_salt() {
        let mut password = SecBuf::with_secure(HASHBYTES);
        let mut pw2_hash = SecBuf::with_secure(HASHBYTES);
        {
            let mut password = password.write_lock();
            password[0] = 42;
            password[1] = 222;
        }
        let mut salt = SecBuf::with_insecure(SALTBYTES);
        hash(
            &mut password,
            OPSLIMIT_SENSITIVE,
            MEMLIMIT_SENSITIVE,
            ALG_ARGON2ID13,
            &mut salt,
            &mut pw2_hash,
        )
        .unwrap();
        let pw2_hash = pw2_hash.read_lock();
        assert_eq!("[84, 166, 168, 46, 130, 222, 122, 144, 123, 49, 206, 167, 35, 180, 246, 154, 25, 43, 218, 177, 95, 218, 12, 241, 234, 207, 230, 93, 127, 174, 221, 106]",  format!("{:?}", *pw2_hash));
    }
    #[test]
    fn it_should_generate_consistantly() {
        let mut password = SecBuf::with_secure(HASHBYTES);
        let mut pw1_hash = SecBuf::with_secure(HASHBYTES);
        let mut pw2_hash = SecBuf::with_secure(HASHBYTES);
        password.randomize();
        let mut salt = SecBuf::with_insecure(SALTBYTES);
        salt.randomize();
        hash(
            &mut password,
            OPSLIMIT_SENSITIVE,
            MEMLIMIT_SENSITIVE,
            ALG_ARGON2ID13,
            &mut salt,
            &mut pw1_hash,
        )
        .unwrap();
        hash(
            &mut password,
            OPSLIMIT_SENSITIVE,
            MEMLIMIT_SENSITIVE,
            ALG_ARGON2ID13,
            &mut salt,
            &mut pw2_hash,
        )
        .unwrap();
        let pw1_hash = pw1_hash.read_lock();
        let pw2_hash = pw2_hash.read_lock();
        assert_eq!(format!("{:?}", *pw1_hash), format!("{:?}", *pw2_hash));
    }

}
