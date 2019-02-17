extern crate bindgen;
extern crate cc;

use std::env;
use std::fs::read_dir;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use cc::Build;

fn main() {

  if !cfg!(windows) {
    assert!(Command::new("git")
      .arg("submodule")
      .arg("init")
      .stdout(Stdio::inherit())
      .stderr(Stdio::inherit())
      .status().unwrap().success(), "git submodule init fail");

    assert!(Command::new("git")
      .args(&["submodule", "update"])
      .stdout(Stdio::inherit())
      .stderr(Stdio::inherit())
      .status().unwrap().success(), "git submodule update fail");
  }

  assert!(Command::new("python")
    .arg("skia/tools/git-sync-deps")
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .status().unwrap().success(), "git sync deps fail");

  let gn_args = {
    let base_args =
      r#"--args=is_official_build=true skia_use_system_expat=false skia_use_system_icu=false skia_use_system_libjpeg_turbo=false skia_use_system_libpng=false skia_use_system_libwebp=false skia_use_system_zlib=false cc="clang" cxx="clang++""#
      .to_owned();

    if cfg!(windows) {
      base_args + r#" clang_win="C:\Program Files\LLVM" extra_cflags=["/MD"]"#
    } else {
      base_args
    }
  };

  let gn_command = if cfg!(windows) {
    "skia/bin/gn"
  } else {
    "bin/gn"
  };

  let skia_out_dir : String =
    PathBuf::from(env::var("OUT_DIR").unwrap())
      .join("skia/Static")
      .to_str().unwrap().into();

  let output = Command::new(gn_command)
    .args(&["gen", &skia_out_dir, &gn_args])
    .envs(env::vars())
    .current_dir(PathBuf::from("./skia"))
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .output()
    .expect("gn error");

  if output.status.code() != Some(0) {
    panic!("{:?}", String::from_utf8(output.stdout).unwrap());
  }

  assert!(Command::new("ninja")
    .current_dir(PathBuf::from("./skia"))
    .args(&["-C", &skia_out_dir])
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .status().unwrap().success(), "ninja error");

  let current_dir = env::current_dir().unwrap();
  let current_dir_name = current_dir.to_str().unwrap();

  println!("cargo:rustc-link-search={}", &skia_out_dir);
  println!("cargo:rustc-link-lib=static=skia");
  println!("cargo:rustc-link-lib=static=skiabinding");

  let target = env::var("TARGET").unwrap();
  if target.contains("unknown-linux-gnu") {
    println!("cargo:rustc-link-lib=stdc++");
    println!("cargo:rustc-link-lib=bz2");
    println!("cargo:rustc-link-lib=GL");
    println!("cargo:rustc-link-lib=fontconfig");
    println!("cargo:rustc-link-lib=freetype");
  } else if target.contains("eabi") {
    println!("cargo:rustc-link-lib=stdc++");
    println!("cargo:rustc-link-lib=GLESv2");
  } else if target.contains("apple-darwin") {
    println!("cargo:rustc-link-lib=c++");
    println!("cargo:rustc-link-lib=framework=OpenGL");
    println!("cargo:rustc-link-lib=framework=ApplicationServices");
  } else if target.contains("windows") {
    if target.contains("gnu") {
      println!("cargo:rustc-link-lib=stdc++");
    }
    println!("cargo:rustc-link-lib=usp10");
    println!("cargo:rustc-link-lib=ole32");
    println!("cargo:rustc-link-lib=user32");
  }

  bindgen_gen(&current_dir_name, &skia_out_dir);
}

fn bindgen_gen(current_dir_name: &str, skia_out_dir: &str) {

  let mut builder = bindgen::Builder::default()
    .generate_inline_functions(true)

    .whitelist_function("SkiaCreateCanvas")
    .whitelist_function("SkiaCreateRect")
    .whitelist_function("SkiaClearCanvas")
    .whitelist_function("SkiaGetSurfaceData")
    .whitelist_var("SK_ColorTRANSPARENT")
    .whitelist_var("SK_ColorBLACK")
    .whitelist_var("SK_ColorDKGRAY")
    .whitelist_var("SK_ColorGRAY")
    .whitelist_var("SK_ColorLTGRAY")
    .whitelist_var("SK_ColorWHITE")
    .whitelist_var("SK_ColorRED")
    .whitelist_var("SK_ColorGREEN")
    .whitelist_var("SK_ColorBLUE")
    .whitelist_var("SK_ColorYELLOW")
    .whitelist_var("SK_ColorCYAN")
    .whitelist_var("SK_ColorMAGENTA")
    .use_core()
    .clang_arg("-std=c++14");

  let mut cc_build = Build::new();

  builder = builder.header("src/bindings.cpp");

  for include_dir in read_dir("skia/include").expect("Unable to read skia/include") {
    let dir = include_dir.unwrap();
    let include_path = format!("{}/{}", &current_dir_name, &dir.path().to_str().unwrap());
    builder = builder.clang_arg(format!("-I{}", &include_path));
    cc_build.include(&include_path);
  }

  cc_build
    .cpp(true)
    .flag("-std=c++14")
    .file("src/bindings.cpp")
    .out_dir(skia_out_dir)
    .compile("skiabinding");

  let bindings = builder.generate().expect("Unable to generate bindings");

  let out_path = PathBuf::from("src");
  bindings
    .write_to_file(out_path.join("bindings.rs"))
    .expect("Couldn't write bindings!");
}
