use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{DeriveInput, Ident};

#[proc_macro_derive(Task, attributes(taskee))]
pub fn derive_task(input: TokenStream) -> TokenStream {
  let input = syn::parse_macro_input!(input as DeriveInput);
  let vis = &input.vis;
  let payload = &input.ident;
  let task = format_ident!("{}Task", payload);

  let mut status = false;
  let mut progress = false;

  for attr in input.attrs.iter().filter(|attr| attr.path().is_ident("taskee")) {
    if let Err(e) = attr.parse_nested_meta(|meta| {
      if meta.path.is_ident("status") {
        status = true;
      } else if meta.path.is_ident("progress") {
        progress = true;
      } else {
        return Err(meta.error(format!("unknown attribute `{}`", meta.path.to_token_stream())));
      }
      Ok(())
    }) {
      return e.to_compile_error().into();
    }
  }

  let status_impl = if status {
    let status_type = if let syn::Data::Struct(data_struct) = &input.data {
      data_struct.fields.iter().find(|f| f.ident.as_ref().is_some_and(|i| i == "status")).map(|f| f.ty.clone())
    } else {
      None
    };

    if let Some(status_type) = status_type {
      let impl_macro = format_ident!("__impl_{}_methods", status_type.to_token_stream().to_string().to_lowercase());
      quote! {
        #impl_macro!(#task);
      }
    } else {
      quote! {}
    }
  } else {
    quote! {}
  };

  let progress_impl = if progress {
    quote! {
      impl taskee::Progress for #payload {
        fn current(&self) -> u64 { self.current }
        fn total(&self) -> u64 { self.total }
        fn progress(&self) -> u8 { self.progress }
        fn set_current(&mut self, value: u64) { self.current = value; }
        fn set_total(&mut self, value: u64) { self.total = value; }
        fn set_progress(&mut self, value: u8) { self.progress = value; }
      }
    }
  } else {
    quote! {}
  };

  quote! {
    #[derive(Clone)]
    #vis struct #task(taskee::Task<#payload>);

    impl taskee::WrapTask<#payload> for #task {
      fn from_task(task: taskee::Task<#payload>) -> Self {
        Self(task)
      }
    }

    impl #task {
      #vis async fn new(id: impl Into<String>) -> (Self, taskee::TaskHandle<#payload>) {
        taskee::Task::<#payload>::new_wrapped::<Self>(id).await
      }

      #vis async fn close(self) {
        self.0.close().await;
      }
    }

    impl std::ops::Deref for #task {
      type Target = taskee::Task<#payload>;
      fn deref(&self) -> &Self::Target {
        &self.0
      }
    }

    #status_impl
    #progress_impl
  }
  .into()
}

struct StatusVariant {
  name: Ident,
  sname: Ident,
  is_terminal: bool,
  has_fields: bool,
}

#[proc_macro_derive(TaskStatus, attributes(terminal))]
pub fn derive_task_status(input: TokenStream) -> TokenStream {
  let input = syn::parse_macro_input!(input as DeriveInput);
  let status = &input.ident;

  let syn::Data::Enum(data) = &input.data else {
    return syn::Error::new_spanned(&input, "TaskStatus can only be derived for enums").to_compile_error().into();
  };

  let variants: Vec<_> = data
    .variants
    .iter()
    .map(|v| {
      let has_fields = !matches!(v.fields, syn::Fields::Unit);
      let sname = format_ident!("{}", snake_case(&v.ident.to_string()));

      StatusVariant {
        name: v.ident.clone(),
        sname,
        is_terminal: v.attrs.iter().any(|a| a.path().is_ident("terminal")),
        has_fields,
      }
    })
    .collect();
  let terminal_variants: Vec<_> = variants.iter().filter(|v| v.is_terminal).collect();
  let non_terminal_variants: Vec<_> = variants.iter().filter(|v| !v.is_terminal && !v.has_fields).collect();

  let terminal_matches: Vec<_> = terminal_variants
    .iter()
    .map(|v| {
      let variant = &v.name;
      quote! { Self::#variant { .. } => true }
    })
    .collect();

  let terminal_check = if terminal_matches.is_empty() {
    quote! { false }
  } else {
    quote! {
      match self {
        #(#terminal_matches,)*
        _ => false,
      }
    }
  };

  let setter_methods: Vec<_> = variants
    .iter()
    .filter_map(|v| {
      let variant = &v.name;
      let method = &v.sname;

      if v.is_terminal {
        if !v.has_fields {
          Some(quote! {
            pub fn #method(&mut self) {
              *self = Self::#variant;
            }
          })
        } else {
          None
        }
      } else if !v.has_fields {
        let setter = format_ident!("set_{}", method);
        Some(quote! {
          pub fn #setter(&mut self) {
            *self = Self::#variant;
          }
        })
      } else {
        None
      }
    })
    .collect();

  let non_terminal_method_rules: Vec<_> = non_terminal_variants
    .iter()
    .map(|v| {
      let variant = &v.name;
      let method = format_ident!("set_{}", v.sname);
      let doc = format!("Sets the task status to `{}`.", variant);
      quote! {
        #[doc = #doc]
        pub async fn #method(&self) {
          self.0.update(|payload| {
            payload.status = #status::#variant;
            true
          }).await;
        }
      }
    })
    .collect();

  let terminal_method_rules: Vec<_> = terminal_variants
    .iter()
    .filter_map(|v| {
      let variant = &v.name;
      let method = &v.sname;
      let doc = format!("Sets the status to `{}` and closes the task.", variant);

      if !v.has_fields {
        Some(quote! {
          #[doc = #doc]
          pub async fn #method(self) {
            self.0.update(|payload| {
              payload.status = #status::#variant;
              true
            }).await;
            self.0.close().await;
          }
        })
      } else {
        None
      }
    })
    .collect();

  let impl_macro = format_ident!("__impl_{}_methods", status.to_string().to_lowercase());

  quote! {
    impl #status {
      pub fn is_terminal(&self) -> bool {
        #terminal_check
      }

      #(#setter_methods)*
    }

    #[doc(hidden)]
    #[macro_export]
    macro_rules! #impl_macro {
      ($task_type:ty) => {
        impl $task_type {
          #(#non_terminal_method_rules)*
          #(#terminal_method_rules)*
        }
      };
    }
  }
  .into()
}

fn snake_case(s: &str) -> String {
  s.chars().enumerate().fold(String::new(), |mut acc, (i, c)| {
    if c.is_uppercase() {
      if i > 0 {
        acc.push('_');
      }
      acc.push(c.to_ascii_lowercase());
    } else {
      acc.push(c);
    }
    acc
  })
}
