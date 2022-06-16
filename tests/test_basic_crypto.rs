use tardis::basic::result::TardisResult;
use tardis::TardisFuns;

#[tokio::test]
async fn test_basic_crypto() -> TardisResult<()> {
    let b64_str = TardisFuns::crypto.base64.encode("测试");
    let str = TardisFuns::crypto.base64.decode(&b64_str)?;
    assert_eq!(str, "测试");

    assert_eq!(TardisFuns::crypto.digest.md5("测试")?, "db06c78d1e24cf708a14ce81c9b617ec");
    assert_eq!(TardisFuns::crypto.digest.sha1("测试")?, "0b5d7ed54bee16756a7579c6718ab01e3d1b75eb");
    assert_eq!(
        TardisFuns::crypto.digest.sha256("测试")?,
        "6aa8f49cc992dfd75a114269ed26de0ad6d4e7d7a70d9c8afb3d7a57a88a73ed"
    );
    assert_eq!(
        TardisFuns::crypto.digest.sha512("测试")?,
        "98fb26ea83ce0f08918c967392a26ab298740aff3c18d032983b88bcee2e16d152ef372778259ebd529ed01701ff01ac4c95ed94e3a1ab9272ab98daf11f076c"
    );
    assert_eq!(TardisFuns::crypto.digest.hmac_sha1("测试", "pwd")?, "0e+vxZN90mgzsju6KCbS2EJ8Us4=");
    assert_eq!(TardisFuns::crypto.digest.hmac_sha256("测试", "pwd")?, "4RnnEGA9fWaf/4mnWSQbJsdtsCXeXdUddSZUmXe6qn4=");
    assert_eq!(
        TardisFuns::crypto.digest.hmac_sha512("测试", "pwd")?,
        "wO2937bb3tY/zLxUped257He0QMWywTsyhf2ELB3YWJmCgN4rS5a6+yWS852MC1LZ5HRd3AQjlUSOUUYKk0p9w=="
    );

    let large_text = "为什么选择 Rust?
高性能
Rust 速度惊人且内存利用率极高。由于没有运行时和垃圾回收，它能够胜任对性能要求特别高的服务，可以在嵌入式设备上运行，还能轻松和其他语言集成。

可靠性
Rust 丰富的类型系统和所有权模型保证了内存安全和线程安全，让您在编译期就能够消除各种各样的错误。

生产力
Rust 拥有出色的文档、友好的编译器和清晰的错误提示信息， 还集成了一流的工具——包管理器和构建工具， 智能地自动补全和类型检验的多编辑器支持， 以及自动格式化代码等等。";

    // AES

    // let key = "4ef240e99f000781c42f4993ddbc996b0964d833d349759685f9e6a1efe84b9c";
    // let iv = "d26efdfa65ee465ec36e847dd9f63ddd";

    // let key = TardisFuns::crypto.key.rand_32_hex()?;
    // let iv = TardisFuns::crypto.key.rand_16_hex()?;

    let key = TardisFuns::crypto.key.rand_16_hex()?;
    let iv = TardisFuns::crypto.key.rand_16_hex()?;

    let encrypted_data = TardisFuns::crypto.aes.encrypt_cbc(large_text, &key, &iv)?;
    let data = TardisFuns::crypto.aes.decrypt_cbc(&encrypted_data, &key, &iv)?;
    assert_eq!(data, large_text);

    // RSA

    let private_key = TardisFuns::crypto.rsa.new_private_key(2048)?;
    let private_key_pem = private_key.serialize()?;
    let private_key_pem_copy = TardisFuns::crypto.rsa.new_private_key_from_str(private_key_pem.as_str())?.serialize()?;
    assert_eq!(private_key_pem_copy, private_key_pem);

    let public_key1 = TardisFuns::crypto.rsa.new_public_key(&private_key)?;
    let public_key2 = TardisFuns::crypto.rsa.new_public_key_from_public_key(public_key1.serialize()?.as_str())?;
    let public_key3 = TardisFuns::crypto.rsa.new_public_key_from_private_key(private_key_pem.as_str())?;

    let signed_data = private_key.sign("测试")?;
    assert!(public_key1.verify("测试", &signed_data)?);
    assert!(public_key2.verify("测试", &signed_data)?);
    assert!(public_key3.verify("测试", &signed_data)?);
    assert!(!public_key3.verify("测试1", &signed_data)?);

    let encrypted_data = public_key1.encrypt("测试")?;
    assert_eq!(private_key.decrypt(&encrypted_data)?, "测试");

    let encrypted_data = public_key2.encrypt("测试")?;
    assert_eq!(private_key.decrypt(&encrypted_data)?, "测试");

    let encrypted_data = public_key3.encrypt("测试")?;
    assert_eq!(private_key.decrypt(&encrypted_data)?, "测试");

    // SM3

    assert_eq!(TardisFuns::crypto.digest.sm3("测试")?, "6fcf886a3115eb3b18d2dba1b4413fed5067c154e030276d8a78caa773b44eab");
    assert_eq!(
        TardisFuns::crypto.digest.sm3(large_text)?,
        "06717d16b797096e5050adb2f8c2daabf4d8f26d5c3a8da5c6171bec2becb497"
    );

    // SM4

    let key = TardisFuns::crypto.key.rand_16_hex()?;
    let iv = TardisFuns::crypto.key.rand_16_hex()?;

    let encrypted_data = TardisFuns::crypto.sm4.encrypt_cbc(large_text, &key, &iv)?;
    let data = TardisFuns::crypto.sm4.decrypt_cbc(&encrypted_data, &key, &iv)?;
    assert_eq!(data, large_text);

    // SM2

    let private_key = TardisFuns::crypto.sm2.new_private_key()?;
    let private_key_pem = private_key.serialize()?;
    let private_key_pem_copy = TardisFuns::crypto.sm2.new_private_key_from_str(private_key_pem.as_str())?.serialize()?;
    assert_eq!(private_key_pem_copy, private_key_pem);

    let public_key1 = TardisFuns::crypto.sm2.new_public_key(&private_key)?;
    let public_key2 = TardisFuns::crypto.sm2.new_public_key_from_public_key(public_key1.serialize()?.as_str())?;
    let public_key3 = TardisFuns::crypto.sm2.new_public_key_from_private_key(private_key_pem.as_str())?;

    let signed_data = private_key.sign("测试")?;
    assert!(public_key1.verify("测试", &signed_data)?);
    assert!(public_key2.verify("测试", &signed_data)?);
    assert!(public_key3.verify("测试", &signed_data)?);
    assert!(!public_key3.verify("测试1", &signed_data)?);

    let encrypted_data = public_key1.encrypt("测试")?;
    assert_eq!(private_key.decrypt(&encrypted_data)?, "测试");

    let encrypted_data = public_key2.encrypt("测试")?;
    assert_eq!(private_key.decrypt(&encrypted_data)?, "测试");

    let encrypted_data = public_key3.encrypt("测试")?;
    assert_eq!(private_key.decrypt(&encrypted_data)?, "测试");

    let signed_data = private_key.sign(large_text)?;
    assert!(public_key1.verify(large_text, &signed_data)?);
    let encrypted_data = public_key1.encrypt(large_text)?;
    assert_eq!(private_key.decrypt(&encrypted_data)?, large_text);

    Ok(())
}
