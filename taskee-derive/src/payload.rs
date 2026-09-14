use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Error, Fields, Ident, Result, Type, spanned::Spanned};

#[derive(Default)]
struct Options {
  status: Option<Span>,
  progress: Option<Span>,
}

fn parse_options(input: &DeriveInput) -> Result<Options> {
  let mut options = Options::default();

  for attr in input.attrs.iter().filter(|attr| attr.path().is_ident("taskee")) {
    attr.parse_nested_meta(|meta| {
      let span = meta.path.span();
      if meta.path.is_ident("status") {
        options.status = Some(span);
      } else if meta.path.is_ident("progress") {
        options.progress = Some(span);
      } else {
        return Err(meta.error("unknown `taskee` option, expected `status` or `progress`"));
      }
      Ok(())
    })?;
  }

  Ok(options)
}

fn named_fields(input: &DeriveInput) -> Result<&syn::FieldsNamed> {
  match &input.data {
    Data::Struct(data) => match &data.fields {
      Fields::Named(fields) => Ok(fields),
      _ => Err(Error::new_spanned(input, "`Task` requires a struct with named fields")),
    },
    _ => Err(Error::new_spanned(input, "`Task` can only be derived for structs")),
  }
}

fn field_type<'a>(fields: &'a syn::FieldsNamed, name: &Ident) -> Option<&'a Type> {
  fields.named.iter().find(|field| field.ident.as_ref().is_some_and(|ident| ident == name)).map(|field| &field.ty)
}

fn require_fields(fields: &syn::FieldsNamed, required: &[&Ident], span: Span, what: &str) -> Result<()> {
  let missing: Vec<String> =
    required.iter().filter(|name| field_type(fields, name).is_none()).map(|name| name.to_string()).collect();

  if missing.is_empty() {
    return Ok(());
  }
  Err(Error::new(span, format!("`{what}` needs the payload to have the field(s): {}", missing.join(", "))))
}

pub(crate) fn expand(input: DeriveInput) -> Result<TokenStream> {
  let options = parse_options(&input)?;
  let fields = named_fields(&input)?;

  let payload = &input.ident;
  let visibility = &input.vis;
  let task = format_ident!("{}Task", payload);

  if !input.generics.params.is_empty() {
    return Err(Error::new_spanned(&input.generics, "`Task` does not support generic payloads"));
  }

  let status_impl = match options.status {
    None => quote! {},
    Some(span) => {
      let status = Ident::new("status", span);
      require_fields(fields, &[&status], span, "taskee(status)")?;
      let status_type = field_type(fields, &status).expect("checked above");

      quote! {
        impl ::taskee::HasStatus for #payload {
          type Status = #status_type;
          fn status(&self) -> &Self::Status { &self.status }
          fn set_status(&mut self, status: Self::Status) { self.status = status; }
        }
      }
    }
  };

  let progress_impl = match options.progress {
    None => quote! {},
    Some(span) => {
      let fields_needed = ["current", "total", "progress"].map(|name| Ident::new(name, span));
      require_fields(fields, &fields_needed.iter().collect::<Vec<_>>(), span, "taskee(progress)")?;

      quote! {
        impl ::taskee::Progress for #payload {
          fn current(&self) -> u64 { self.current }
          fn total(&self) -> u64 { self.total }
          fn progress(&self) -> u8 { self.progress }
          fn set_current(&mut self, current: u64) { self.current = current; }
          fn set_total(&mut self, total: u64) { self.total = total; }
          fn set_progress(&mut self, progress: u8) { self.progress = progress; }
        }
      }
    }
  };

  let task_doc = format!("A task writing [`{payload}`] state.");
  let new_doc = format!("Creates a `{task}` and the first handle observing it.");

  Ok(quote! {
    #[doc = #task_doc]
    #[derive(Clone)]
    #visibility struct #task(::taskee::Task<#payload>);

    #[allow(dead_code)]
    impl #task {
      #[doc = #new_doc]
      pub fn new(id: impl Into<String>) -> (Self, ::taskee::TaskHandle<#payload>) {
        let (task, handle) = ::taskee::Task::<#payload>::new(id);
        (Self(task), handle)
      }
    }

    impl ::taskee::AsTask for #task {
      type Payload = #payload;
      fn as_task(&self) -> &::taskee::Task<#payload> { &self.0 }
      fn into_task(self) -> ::taskee::Task<#payload> { self.0 }
    }

    impl ::core::ops::Deref for #task {
      type Target = ::taskee::Task<#payload>;
      fn deref(&self) -> &Self::Target {
        &self.0
      }
    }

    #status_impl
    #progress_impl
  })
}
