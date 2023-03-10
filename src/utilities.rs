use llvm_ir::TypeRef;

const ALPHABET_SIZE: usize = 26;
const LOWERCASE_A_OFFSET: usize = 97;
pub fn usize_to_string(num: usize) -> String {
    let idx = (num) / ALPHABET_SIZE;
    let ascii_code = ((num % ALPHABET_SIZE) + LOWERCASE_A_OFFSET) as u8;
    let ascii_char = ascii_code as char;
    if idx > 0 {
        format!("{}{}", ascii_char, idx)
    } else {
        format!("{}", ascii_char)
    }
}

pub fn dereference_type(tr: &TypeRef) -> Option<&TypeRef> {
    match tr.as_ref() {
        llvm_ir::Type::PointerType {
            pointee_type,
            addr_space: _,
        } => Some(pointee_type),
        _ => None,
    }
}
