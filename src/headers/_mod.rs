#![cfg_attr(rustfmt, rustfmt::skip)]
//! C headers generation.
//!
//! This module is only enabled when the `"headers"` feature of `::safer_ffi` is
//! enabled, which is expected to be done through a cargo feature within the
//! (downstream) crate defining the `#[ffi_export]`ed
//! functions.
//!
//! ```toml
//! [dependencies]
//! safer-ffi = { version = "...", features = ["proc_macros"] }
//!
//! [features]
//! generate-headers = ["safer-ffi/headers"]
//! ```
//!
//! Then, to generate the bindings, just define a
//! `#[safer_ffi::cfg_headers]`-gated `#[test]` function,
//! which can then call the [`builder`] to do the work:
//!
//! ```rust
//! use ::std::{io, fs};
//! use ::safer_ffi::prelude::*;
//!
//! /// Concatenate two strings.
//! ///
//! /// The returned value must be freed with `rust_free`
//! #[ffi_export]
//! fn rust_concat (fst: char_p::Ref<'_>, snd: char_p::Ref<'_>)
//!   -> char_p::Box
//! {
//!     let s: String = format!("{}{}\0", fst, snd);
//!     s   .try_into() // Try to convert to a boxed `char *` pointer
//!         .unwrap()   // Only fails if there is an inner nul byte.
//! }
//!
//! /// Frees a pointer obtained by calling `rust_concat`.
//! #[ffi_export]
//! fn rust_free (it: char_p::Box)
//! {
//!     drop(it);
//! }
//!
//! # #[cfg(any())] macro_rules! {
//! #[::safer_ffi::cfg_headers]
//! #[test]
//! # }
//! fn generate_c_header ()
//!   -> io::Result<()>
//! {
//!     ::safer_ffi::headers::builder()
//!         .with_guard("__ASGARD__")
//!         .to_file("filename.h")?
//!         .generate()
//! }
//! # generate_c_header().unwrap();
//! ```
//!
//! so that
//!
//! ```shell
//! cargo test --features generate-headers -- \
//!     --exact generate_c_header \
//!     --nocapture
//! ```
//!
//! generates a `"filename.h"` file (⚠️ overwriting it if it exists ⚠️) with
//! the following contents:
//!
//! <pre style="color:#000020;background:#f6f8ff;"><span style="color:#3f7f8f; ">/*! \file */</span>
//! <span style="color:#3f7f8f; ">/*******************************************</span>
//! <span style="color:#3f7f8f; ">&nbsp;*                                         *</span>
//! <span style="color:#3f7f8f; ">&nbsp;*  File auto-generated by `::safer_ffi`.  *</span>
//! <span style="color:#3f7f8f; ">&nbsp;*                                         *</span>
//! <span style="color:#3f7f8f; ">&nbsp;*  Do not manually edit this file.        *</span>
//! <span style="color:#3f7f8f; ">&nbsp;*                                         *</span>
//! <span style="color:#3f7f8f; ">&nbsp;*******************************************/</span>
//!
//! <span style="color:#004a43; ">#</span><span style="color:#004a43; ">ifndef</span><span style="color:#004a43; "> __ASGARD__</span>
//! <span style="color:#004a43; ">#</span><span style="color:#004a43; ">define</span><span style="color:#004a43; "> __ASGARD__</span>
//!
//!
//! <span style="color:#3f7f8f; ">/** \brief</span>
//! <span style="color:#3f7f8f; ">&nbsp;*  Concatenate two strings.</span>
//! <span style="color:#3f7f8f; ">&nbsp;* </span>
//! <span style="color:#3f7f8f; ">&nbsp;*  The returned value must be freed with `rust_free_string`</span>
//! <span style="color:#3f7f8f; ">&nbsp;*/</span>
//! <span style="color:#200080; font-weight:bold; ">char</span> <span style="color:#308080; ">*</span> rust_concat <span style="color:#308080; ">(</span>
//!     <span style="color:#200080; font-weight:bold; ">char</span> <span style="color:#200080; font-weight:bold; ">const</span> <span style="color:#308080; ">*</span> fst<span style="color:#308080; ">,</span>
//!     <span style="color:#200080; font-weight:bold; ">char</span> <span style="color:#200080; font-weight:bold; ">const</span> <span style="color:#308080; ">*</span> snd<span style="color:#308080; ">)</span><span style="color:#406080; ">;</span>
//!
//! <span style="color:#3f7f8f; ">/** \brief</span>
//! <span style="color:#3f7f8f; ">&nbsp;*  Frees a pointer obtained by calling `rust_concat`.</span>
//! <span style="color:#3f7f8f; ">&nbsp;*/</span>
//! <span style="color:#200080; font-weight:bold; ">void</span> rust_free_string <span style="color:#308080; ">(</span>
//!     <span style="color:#200080; font-weight:bold; ">char</span> <span style="color:#308080; ">*</span> it<span style="color:#308080; ">)</span><span style="color:#406080; ">;</span>
//!
//!
//! <span style="color:#004a43; ">#</span><span style="color:#004a43; ">endif</span><span style="color:#004a43; "> </span><span style="color:#595979; ">/* __ASGARD__ */</span>
//! </pre>

#![allow(missing_copy_implementations, missing_debug_implementations)]

use ::std::{
    collections::HashSet,
    fs,
    io,
    path::Path,
};

use_prelude!();
use rust::{String};

pub // (in crate)
mod languages;

pub use definer::{Definer, HashSetDefiner};
mod definer;



match_! {(
    /// Sets up the name of the `ifndef` guard of the header file.
    ///
    /// It defaults to:
    ///
    /// ```rust,ignore
    /// format!("__RUST_{}__", env::var("CARGO_CRATE_NAME")?.replace("-", "_").to_ascii_uppercase())
    /// ```
    guard: &'__ str,

    /// Sets up the banner of the generated C header file.
    ///
    /// It defaults to:
    ///
    /// ```rust,ignore
    /// concat!(
    ///     "/*! \\file */\n",
    ///     "/*******************************************\n",
    ///     " *                                         *\n",
    ///     " *  File auto-generated by `::safer_ffi`.  *\n",
    ///     " *                                         *\n",
    ///     " *  Do not manually edit this file.        *\n",
    ///     " *                                         *\n",
    ///     " *******************************************/\n",
    /// )
    /// ```
    ///
    /// <pre style="color:#000020;background:#f6f8ff;"><span style="color:#3f7f8f; ">/*! \file */</span>
    /// <span style="color:#3f7f8f; ">/*******************************************</span>
    /// <span style="color:#3f7f8f; ">&nbsp;*                                         *</span>
    /// <span style="color:#3f7f8f; ">&nbsp;*  File auto-generated by `::safer_ffi`.  *</span>
    /// <span style="color:#3f7f8f; ">&nbsp;*                                         *</span>
    /// <span style="color:#3f7f8f; ">&nbsp;*  Do not manually edit this file.        *</span>
    /// <span style="color:#3f7f8f; ">&nbsp;*                                         *</span>
    /// <span style="color:#3f7f8f; ">&nbsp;*******************************************/</span>
    /// </pre>
    banner: &'__ str,

    /// Sets the [`Language`] of the generated headers.
    ///
    /// It defaults to [`Language::C`].
    language: Language,

    /// Sets prefix for generated functions, structs & enums
    naming_convention: NamingConvention,

    /// Whether to yield a stable header or not (order of defined items guaranteed
    /// not to change provided the source code doesn't change either).
    ///
    /// It defaults to `true`.
    stable_header: bool,
) /* as */ {(
    $(
        $(#[$field_meta:meta])*
        $field:ident : $field_ty:ty
    ),* $(,)?
) => (
    #[derive(Default)]
    pub
    struct Builder<'__, W> {
        target: W,
        $(
            $field : Option<$field_ty>,
        )*
    }

    pub
    fn builder<'__> ()
      -> Builder<'__, WhereTo>
    {
        Builder::default()
    }

    use __::WhereTo;
    mod __ {
        #[derive(Default)]
        pub
        struct WhereTo;
    }

    ::paste::item! {
        impl<'__, W> Builder<'__, W> {
            $(
                $(#[$field_meta])*
                pub
                fn [<with_$field>] (self, $field : $field_ty)
                  -> Self
                {
                    let $field = Some($field);
                    Self {
                        $field,
                        .. self
                    }
                }
            )*
        }
    }

    impl<'__> Builder<'__, WhereTo> {
        /// Specify the path to the file to be generated.
        ///
        /// **⚠️ If it already exists, its contents will be overwritten ⚠️**
        ///
        /// There is no default value here, either `.to_file()` or [`.to_writer()`]
        /// need to be called to be able to [`.generate()`] the headers.
        ///
        /// For more fine-grained control over the "output stream" where the
        /// headers will be written to, use [`.to_writer()`].
        ///
        /// # Example
        ///
        /// ```rust,no_run
        /// # fn main () -> ::std::io::Result<()> { Ok({
        /// ::safer_ffi::headers::builder()
        ///     .to_file("my_header.h")?
        ///     .generate()?
        /// # })}
        /// ```
        ///
        /// [`.to_writer()`]: `Builder::to_writer`
        /// [`.generate()`]: `Builder::generate`
        pub
        fn to_file (
            self: Self,
            filename: impl AsRef<Path>,
        ) -> io::Result<Builder<'__, fs::File>>
        {
            Ok(self.to_writer(
                fs::OpenOptions::new()
                    .create(true)/*or*/.truncate(true)
                    .write(true)
                    .open(filename.as_ref())?
            ))
        }

        /// Specify the [`Write`][`io::Write`] "stream" where the headers will
        /// be written to.
        ///
        /// # Example
        ///
        /// ```rust,no_run
        /// // Display the headers to the standard output
        /// // (may need the `--nocapture` flag when running the tests)
        /// # fn main () -> ::std::io::Result<()> { Ok({
        /// ::safer_ffi::headers::builder()
        ///     .to_writer(::std::io::stdout())
        ///     .generate()?
        /// # })}
        /// ```
        pub
        fn to_writer<W> (
            self: Self,
            out: W,
        ) -> Builder<'__, W>
        where
            W : io::Write
        {
            let Self {
                target: WhereTo, $(
                $field, )*
                ..
            } = self;
            Builder {
                target: out,
                $($field ,)*
            }
        }
    }

    impl<'__, W : io::Write> Builder<'__, W> {
        /// Generate the C header file.
        pub
        fn generate (self)
          -> io::Result<()>
        {
            let Self { mut target, $($field ,)* } = self;
            Builder {
                target: WhereTo, $(
                $field, )*
            }.generate_with_definer(&mut HashSetDefiner {
                out: &mut target,
                defines_set: Default::default(),
            })
        }

        // pub
        // fn as_mut_dyn (self: &'__ mut Self)
        //   -> Builder<'__, &'__ mut dyn io::Write>
        // where
        //     W : '__,
        // {
        //     let Self { ref mut target, $($field ,)* } = *self;
        //     Builder {
        //         target, $(
        //         $field, )*
        //     }
        // }
    }
)}}

impl Builder<'_, WhereTo> {
    /// More customizable version of [`.generate()`][Builder::generate].
    ///
    /// With this call, one can provide a custom implementation of a [`Definer`],
    /// which can be useful for mock tests, mainly.
    pub
    fn generate_with_definer (self, definer: &mut impl Definer)
      -> io::Result<()>
    {
        let config = self;
        // Banner
        config.write_banner(definer)?;
        // Prelude
        config.write_prelude(definer)?;
        /* User-provided defs! */
        config.write_body(definer)?;
        // Epilogue
        config.write_epilogue(definer)?;
        Ok(())
    }

    fn write_banner (&'_ self, definer: &'_ mut dyn Definer)
      -> io::Result<()>
    {
        let banner: &'_ str = self.banner.unwrap_or(concat!(
            "/*! \\file */\n",
            "/*******************************************\n",
            " *                                         *\n",
            " *  File auto-generated by `::safer_ffi`.  *\n",
            " *                                         *\n",
            " *  Do not manually edit this file.        *\n",
            " *                                         *\n",
            " *******************************************/\n",
        ));
        writeln!(definer.out(), "{}", banner)
    }

    fn write_prelude (&'_ self, definer: &'_ mut dyn Definer)
      -> io::Result<()>
    {
        let lang = self.language.unwrap_or(Language::C);

        let guard = self.guard();

        match lang {
            | Language::C => writeln!(definer.out(),
                include_str!("templates/c/_prelude.h"),
                guard = guard,
            ),

            | Language::CSharp => writeln!(definer.out(),
                include_str!("templates/csharp/_prelude.cs"),
                NameSpace = Self::pascal_cased_lib_name(),
                RustLib = Self::lib_name(),
            ),

            #[cfg(feature = "python-headers")]
            // CHECKME
            | Language::Python => Ok(()),
        }
    }

    /// Heart of safer ffi: write the items in the header
    fn write_body (&'_ self, definer: &'_ mut dyn Definer)
      -> io::Result<()>
    {
        let stable_header = self.stable_header.unwrap_or(true);
        let lang = self.language.unwrap_or(Language::C);
        let _naming_convention =
            self.naming_convention
                .as_ref()
                .unwrap_or(&NamingConvention::Default)
        ;
        let (mut storage0, mut storage1) = (None, None);
        let gen_defs: &mut dyn Iterator<Item = _> = if stable_header {
            storage0.get_or_insert(
                crate::inventory::iter
                    .into_iter()
                    .map(|crate::FfiExport { name, gen_def }| (name, gen_def))
                    // Sort the definitions for a reliable header generation.
                    .collect::<::std::collections::BTreeMap<_, _>>()
                    .into_iter()
                    .map(|(_, gen_def)| gen_def)
            )
        } else {
            storage1.get_or_insert(
                crate::inventory::iter
                    .into_iter()
                    // Iterate in reverse fashion to more closely match
                    // the Rust definition order.
                    .collect::<rust::Vec<_>>().into_iter().rev()
                    .map(|crate::FfiExport { gen_def, .. }| gen_def)
            )
        };
        (&mut { gen_defs }).try_for_each(|gen_def| gen_def(definer, lang))?;
        Ok(())
    }

    fn write_epilogue (&'_ self, definer: &'_ mut dyn Definer)
      -> io::Result<()>
    {
        let lang = self.language.unwrap_or(Language::C);
        match lang {
            | Language::C => write!(definer.out(),
                include_str!("templates/c/epilogue.h"),
                guard = self.guard(),
            ),

            | Language::CSharp => {
                let pkg_name = Self::pascal_cased_lib_name();
                    write!(definer.out(),
                include_str!("templates/csharp/epilogue.cs"),
                PkgName = pkg_name,
            )
            },
            #[cfg(feature = "python-headers")]
            // CHECKME
            | Language::Python => Ok(()),
        }
    }

    fn guard (&'_ self)
      -> String
    {
        self.guard.map_or_else(
            || format!("__RUST_{}__", Self::lib_name().to_ascii_uppercase()),
            Into::into,
        )
    }

    /// Return the library name
    fn lib_name ()
      -> String
    {
        ::std::env::var("CARGO_CRATE_NAME")
            .or_else(|_| {
                ::std::env::var("CARGO_PKG_NAME")
                    .map(|s| s.replace('-', "_"))
            })
            .expect("Missing `CARGO_{CRATE,PKG}_NAME` env vars")
    }

    /// Return a Pascal Cased (UpperCamelCase) version of the lib name.
    fn pascal_cased_lib_name() -> String {
        Self::lib_name()
            .chars()
            .filter_map({
                // `true` for PascalCase, `false` for lowerCamelCase.
                let mut underscore = true;
                move |c| Some(match c {
                    | _ if underscore => {
                        underscore = false;
                        c.to_ascii_uppercase()
                    },

                    | '_' => {
                        underscore = true;
                        return None; // continue
                    },

                    | _ => {
                        c
                    },
                })
            })
            .collect::<String>()
    }
}


/// Language of the generated headers.
#[derive(
    Debug,
    Copy, Clone,
    PartialEq, Eq,
)]
pub
enum Language {
    /// C, _lingua franca_ of FFI interop.
    C,

    /// C#
    CSharp,
    /// Python (experimental).
    #[cfg(feature = "python-headers")]
    Python,
}

/// Allow user to specify
pub
enum NamingConvention {
    Default,
    Suffix(String),
    Prefix(String),
    Custom(fn(&str)-> String),
}

hidden_export! {
    /// Invoke the language-specific typedef code for the given type.
    fn __define_self__<T : ReprC> (
        definer: &'_ mut dyn Definer,
        lang: Language,
    ) -> ::std::io::Result<()>
    {
        match lang {
            | Language::C => {
                <T::CLayout as CType>::define_self(&crate::headers::languages::C, definer)
            },
            | Language::CSharp => {
                <T::CLayout as CType>::define_self(&crate::headers::languages::CSharp, definer)
            },
            #[cfg(feature = "python-headers")]
            | Language::Python => {
                <T::CLayout as CType>::define_self(&crate::headers::languages::Python, definer)
            },
        }
    }
}

use self::languages::{
    FunctionArg,
    HeaderLanguage,
    PhantomCType,
};

#[apply(hidden_export)]
fn __define_fn__ (
    definer: &'_ mut dyn Definer,
    lang: Language,
    docs: &'_ [&'_ str],
    fname: &'_ str,
    args: &'_ [FunctionArg<'_>],
    ret_ty: &'_ dyn PhantomCType,
) -> io::Result<()>
{
    let dyn_lang: &dyn HeaderLanguage = match lang {
        | Language::C => &languages::C,
        | Language::CSharp => &languages::CSharp,
        #[cfg(feature = "python-headers")]
        | Language::Python => &languages::Python,
    };
    dyn_lang.emit_function(
        definer,
        docs,
        fname,
        args,
        ret_ty,
    )
}

hidden_export! {
    /// Helpers for the generation of FFI-imported function declarations.
    mod __define_fn__ {
        use super::*;
        use ::std::{
            fmt::Write as _,
            io::Result,
        };

        pub
        fn name (
            out: &'_ mut String,
            f_name: &'_ str,
            lang: Language,
        )
        {
            match lang {
                | Language::C => write!(out,
                    "{} (", f_name.trim(),
                ),

                | Language::CSharp => write!(out,
                    "{} (", f_name.trim(),
                ),
                #[cfg(feature = "python-headers")]
                | Language::Python => write!(out,
                    "{} (", f_name.trim(),
                ),
            }
            .expect("`write!`-ing to a `String` cannot fail")
        }

        pub
        fn arg<Arg : ReprC> (
            out: &'_ mut String,
            arg_name: &'_ str,
            lang: Language,
        )
        {
            if out.ends_with("(").not() {
                out.push_str(",");
            }
            match lang {
                | Language::C => write!(out,
                    "\n    {}",
                    Arg::CLayout::name_wrapping_var(&crate::headers::languages::C, arg_name),
                ),

                | Language::CSharp => write!(out,
                    "\n        {marshaler}{}",
                     Arg::CLayout::name_wrapping_var(&crate::headers::languages::CSharp, arg_name),
                    marshaler =
                        Arg::CLayout::csharp_marshaler()
                            .map(|m| format!("[MarshalAs({})]\n        ", m))
                            .as_deref()
                            .unwrap_or("")
                    ,
                ),
                #[cfg(feature = "python-headers")]
                | Language::Python => write!(out,
                    "\n    {}",
                    Arg::CLayout::name_wrapping_var(&crate::headers::languages::Python, arg_name),
                ),
            }
            .expect("`write!`-ing to a `String` cannot fail")
        }

        pub
        fn ret<Ret : ReprC> (
            definer: &'_ mut dyn Definer,
            lang: Language,
            mut fname_and_args: String,
        ) -> Result<()>
        {
            let out = definer.out();
            match lang {
                | Language::C => {
                    if fname_and_args.ends_with("(") {
                        fname_and_args.push_str("void");
                    }
                    writeln!(out,
                        "{});\n",
                        Ret::CLayout::name_wrapping_var(&crate::headers::languages::C, &fname_and_args),
                    )
                },

                | Language::CSharp => {
                    writeln!(out,
                        concat!(
                            "public unsafe partial class Ffi {{\n    ",
                            "{mb_marshaler}",
                            "[DllImport(RustLib, ExactSpelling = true)] public static unsafe extern\n",
                            "    {});\n",
                            "}}\n",
                        ),
                        Ret::CLayout::name_wrapping_var(&crate::headers::languages::CSharp, &fname_and_args),
                        mb_marshaler =
                            Ret::CLayout::csharp_marshaler()
                                .map(|m| format!("[return: MarshalAs({})]\n    ", m))
                                .as_deref()
                                .unwrap_or("")
                        ,
                    )
                },
                #[cfg(feature = "python-headers")]
                | Language::Python => {
                    if fname_and_args.ends_with("(") {
                        fname_and_args.push_str("void");
                    }
                    writeln!(out,
                        "{});\n",
                        Ret::CLayout::name_wrapping_var(&crate::headers::languages::Python, &fname_and_args),
                    )
                },
            }
        }
    }
}
