use criterion::{criterion_group, criterion_main, Criterion};

use tardis::TardisFuns;

pub fn crypto_process(c: &mut Criterion) {
    c.bench_function("CRYPTO: base64_encode", |b| b.iter(|| TardisFuns::crypto.base64.encode("测试")));
    c.bench_function("CRYPTO: md5", |b| b.iter(|| TardisFuns::crypto.digest.md5("测试").unwrap()));
    c.bench_function("CRYPTO: sha1", |b| b.iter(|| TardisFuns::crypto.digest.sha1("测试").unwrap()));
    c.bench_function("CRYPTO: sha256", |b| b.iter(|| TardisFuns::crypto.digest.sha256("测试").unwrap()));
    c.bench_function("CRYPTO: sha512", |b| b.iter(|| TardisFuns::crypto.digest.sha512("测试").unwrap()));
    c.bench_function("CRYPTO: hmac_sha1", |b| b.iter(|| TardisFuns::crypto.digest.hmac_sha1("测试", "pwd").unwrap()));
    c.bench_function("CRYPTO: hmac_sha256", |b| b.iter(|| TardisFuns::crypto.digest.hmac_sha256("测试", "pwd").unwrap()));
    c.bench_function("CRYPTO: hmac_sha512", |b| b.iter(|| TardisFuns::crypto.digest.hmac_sha512("测试", "pwd").unwrap()));

    let large_text = "为什么选择 Rust?
高性能
Rust 速度惊人且内存利用率极高。由于没有运行时和垃圾回收，它能够胜任对性能要求特别高的服务，可以在嵌入式设备上运行，还能轻松和其他语言集成。

可靠性
Rust 丰富的类型系统和所有权模型保证了内存安全和线程安全，让您在编译期就能够消除各种各样的错误。

生产力
Rust 拥有出色的文档、友好的编译器和清晰的错误提示信息， 还集成了一流的工具——包管理器和构建工具， 智能地自动补全和类型检验的多编辑器支持， 以及自动格式化代码等等。";

    // AES

    let key = TardisFuns::crypto.key.rand_16_hex().unwrap();
    let iv = TardisFuns::crypto.key.rand_16_hex().unwrap();
    let encrypted_data = TardisFuns::crypto.aes.encrypt_cbc(large_text, &key, &iv).unwrap();
    c.bench_function("CRYPTO: aes_encrypt_cbc", |b| {
        b.iter(|| {
            TardisFuns::crypto.aes.encrypt_cbc(large_text, &key, &iv).unwrap();
        })
    });
    c.bench_function("CRYPTO: aes_decrypt_cbc", |b| {
        b.iter(|| {
            TardisFuns::crypto.aes.decrypt_cbc(&encrypted_data, &key, &iv).unwrap();
        })
    });

    // RSA
    let private_key = TardisFuns::crypto.rsa.new_private_key(2048).unwrap();
    let public_key = TardisFuns::crypto.rsa.new_public_key(&private_key).unwrap();
    let signed_data = private_key.sign("测试").unwrap();
    let encrypted_data = public_key.encrypt("测试").unwrap();
    c.bench_function("CRYPTO: rsa_sign", |b| {
        b.iter(|| {
            private_key.sign("测试").unwrap();
        })
    });
    c.bench_function("CRYPTO: rsa_verify", |b| {
        b.iter(|| {
            public_key.verify("测试", &signed_data).unwrap();
        })
    });
    c.bench_function("CRYPTO: rsa_encrypt", |b| {
        b.iter(|| {
            public_key.encrypt("测试").unwrap();
        })
    });
    c.bench_function("CRYPTO: rsa_decrypt", |b| {
        b.iter(|| {
            private_key.decrypt(&encrypted_data).unwrap();
        })
    });

    // SM3
    c.bench_function("CRYPTO: sm3", |b| {
        b.iter(|| {
            TardisFuns::crypto.digest.sm3("测试").unwrap();
        })
    });
    c.bench_function("CRYPTO: sm3_large_text", |b| {
        b.iter(|| {
            TardisFuns::crypto.digest.sm3(large_text).unwrap();
        })
    });

    // SM4
    let key = TardisFuns::crypto.key.rand_16_hex().unwrap();
    let iv = TardisFuns::crypto.key.rand_16_hex().unwrap();
    let encrypted_data = TardisFuns::crypto.sm4.encrypt_cbc(large_text, &key, &iv).unwrap();
    c.bench_function("CRYPTO: sm4_encrypt_cbc", |b| {
        b.iter(|| {
            TardisFuns::crypto.sm4.encrypt_cbc(large_text, &key, &iv).unwrap();
        })
    });
    c.bench_function("CRYPTO: sm4_decrypt_cbc", |b| {
        b.iter(|| {
            TardisFuns::crypto.sm4.decrypt_cbc(&encrypted_data, &key, &iv).unwrap();
        })
    });

    // SM2
    let private_key = TardisFuns::crypto.sm2.new_private_key().unwrap();
    let public_key = TardisFuns::crypto.sm2.new_public_key_from_private_key(private_key.serialize().unwrap().as_str()).unwrap();
    let encrypted_data = public_key.encrypt("测试").unwrap();
    c.bench_function("CRYPTO: sm2_encrypt", |b| {
        b.iter(|| {
            public_key.encrypt("测试").unwrap();
        })
    });
    c.bench_function("CRYPTO: sm2_decrypt", |b| {
        b.iter(|| {
            private_key.decrypt(&encrypted_data).unwrap();
        })
    });
}

criterion_group!(benches, crypto_process);
criterion_main!(benches);
