function with_v8(id, args) {
    return syscall_v8(id, args);
}

function encodeJson(args) {
    const s = JSON.stringify(args);
    return core_encode(s);
}

function decodeJson(ui8) {
    const s = core_decode(ui8);
    return JSON.parse(s);
}

function with_json(id, args) {
    const buf = encodeJson(args);
    const v = syscall_json(id, buf);
    // TODO: re-enable once v8 is serializing returns
    // so they're on equal footing
    // return decodeJson(v);
    return null;
}

function sum(fn, nums) {
    return fn(1, nums);
}

function add(fn, a, b) {
    return fn(2, {a, b});
}

function promote(fn, fname, lname, age) {
    return fn(3, { first_name: fname, last_name: lname, age });
} 
