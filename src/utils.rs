use std::borrow::Cow;

pub fn dv(input: &[u8]) -> Cow<str> {
    String::from_utf8_lossy(&input)
}

pub fn epoch_int() -> u64 {
    use std::time::{UNIX_EPOCH, SystemTime};

    let now = SystemTime::now();
    let unix = now.duration_since(UNIX_EPOCH).unwrap().as_secs();

    unix
}

pub fn split_string(input: &[u8]) -> Vec<Vec<u8>> {
    let mut buf: Vec<Vec<u8>> = Vec::new();
    let mut tmp: Vec<u8> = Vec::new();

    for (index, &i) in input.iter().enumerate() {
        if i == b' ' {
            buf.push(tmp);
            tmp = Vec::new();
            continue;
        }

        tmp.push(i);

        if index + 1 == input.len() {
            buf.push(tmp);
            break;
        }
    }

    buf
}

pub fn unsplit_string(argv: &[Vec<u8>], argc: usize, startidx: usize, max: usize) -> Vec<u8> {
    let mut dest: Vec<u8> = Vec::new();
    let mut vec: Vec<Vec<u8>> = Vec::new();

    for i in startidx..argc {
        vec.push(argv[i as usize].clone());
    }

    if max > vec.len() { return dest; }

    for i in 0..max {
        for j in 0..vec[i as usize].len() {
            dest.push(vec[i as usize][j as usize]);
        }
        dest.push(b' ');
    }

    dest
}

pub fn u8_slice_to_lower(input: &[u8]) -> Vec<u8> {
    use std::ascii::AsciiExt;

    let mut buf: Vec<u8> = input.to_vec().clone();
    for byte in &mut buf {
        byte.make_ascii_lowercase();
    }

    return buf;
}

pub fn trim_bytes_right(mut input: &[u8]) -> &[u8] {
    loop {
        match input.iter().next_back() {
            Some(&b'\r') | Some(&b'\n') => {
                input = &input[0..input.len()-1]
            }
            _ => break,
        }
    }

    input
}

pub fn ceiling_division(left: usize, right: usize) -> usize {
    assert!(left > 0);

    1 + ((left - 1) / right)
}

// 64*64*1    64*1     1*2
// #define NUMNICKLOG 6
// #define NUMNICKBASE (1 << NUMNICKLOG)
// #define NUMNICKMASK (NUMNICKBASE - 1)
pub fn inttobase64(mut v: usize, count: usize) -> String {
    static CONVERT2Y: &'static [u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789[]";

    let mut buf: Vec<u8> = Vec::new();
    for _ in 0..count {
        buf.push(CONVERT2Y[v & ((1 << 6) - 1)]);
        v >>= 6;
    }

    buf.reverse();
    String::from_utf8(buf).unwrap()
}

#[test]
fn test_inttobase64() {
    assert_eq!(&inttobase64(16, 3), "AAQ");
    assert_eq!(&inttobase64(80, 3), "ABQ");
    assert_eq!(&inttobase64(4176, 3), "BBQ");
    assert_eq!(&inttobase64(21399, 3), "FOX");
    assert_eq!(&inttobase64(91397, 3), "WUF");
}

#[test]
fn test_ceiling_division() {
    assert_eq!(ceiling_division(499, 500), 1);
    assert_eq!(ceiling_division(500, 500), 1);
    assert_eq!(ceiling_division(501, 500), 2);
}

#[test]
fn test_unsplit_string() {
    let my_argv: Vec<Vec<u8>> = vec![
        format!("A").into_bytes(),
        format!("BB").into_bytes(),
        format!("TEST").into_bytes(),
        format!("SOMETING").into_bytes(),
        format!("FDSAADFS").into_bytes(),
        format!("FADS").into_bytes(),
        format!("ASDf").into_bytes(),
    ];

    let new_unsplit = unsplit_string(&my_argv, 7, 3, 3);
    assert_eq!(&new_unsplit, b"SOMETING FDSAADFS FADS ");
    assert_eq!(new_unsplit.len(), 23);


    let my_argv: Vec<Vec<u8>> = vec![
        format!("B").into_bytes(),
        format!("#channel").into_bytes(),
        format!("9999999999").into_bytes(),
        format!("+stnzl").into_bytes(),
        format!("554").into_bytes(),
        format!("AAAAA:o,AAAAB,AAAAC").into_bytes(),
    ];

    let new_unsplit = unsplit_string(&my_argv, 6, 3, 2);
    assert_eq!(&new_unsplit, b"+stnzl 554 ");
    assert_eq!(new_unsplit.len(), 11);
}

#[test]
fn test_split_string() {
    let s = split_string(&format!("+ntl 34").into_bytes());
    assert_eq!(s.len(), 2);
    assert_eq!(s[0], b"+ntl");
    assert_eq!(s[1], b"34");

    let s = split_string(&format!("fdas fasd adsf asfd dfas").into_bytes());
    assert_eq!(s.len(), 5);
    assert_eq!(s[0], b"fdas");
    assert_eq!(s[1], b"fasd");
    assert_eq!(s[2], b"adsf");
    assert_eq!(s[3], b"asfd");
    assert_eq!(s[4], b"dfas");

    let s = split_string(&format!("aaaaaaa").into_bytes());
    assert_eq!(s.len(), 1);
    assert_eq!(s[0], b"aaaaaaa");

    let s = split_string(&format!("").into_bytes());
    assert_eq!(s.len(), 0);
}

#[test]
fn test_u8_slice_to_lower() {
    let caps = &String::from("THIS IS IN ALL CAPS").into_bytes();
    let lowered = u8_slice_to_lower(&caps);
    assert_eq!(lowered.len(), 19);
    assert_eq!(lowered, b"this is in all caps");
}

#[test]
fn test_trim_bytes_right() {
    let mystr: &[u8] = &String::from("This has newlines and a carriage return\r\n").into_bytes();
    let clean = trim_bytes_right(mystr);
    assert_eq!(clean.len(), 39);
    assert_eq!(clean[38], b'n');
}
