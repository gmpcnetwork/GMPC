fn main() {
    println!("semver: {},
target: {},
sha short: {},
Build Timestamp: {}", 
    env!("VERGEN_SHA_SHORT"),
    env!("VERGEN_TARGET_TRIPLE"),
    env!("VERGEN_SEMVER"),
    env!("VERGEN_BUILD_TIMESTAMP"));
    
}
