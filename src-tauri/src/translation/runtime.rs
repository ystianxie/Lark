use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use chrono::{TimeZone, Utc};
use crypto::hmac::Hmac;
use crypto::digest::Digest;
use crypto::mac::Mac;
use crypto::sha2::Sha256;
use super::model::{TranslationRequest, TranslationResponse};


