use syn::Ident;

pub(crate) fn snake_case(name: &str) -> String {
  let chars: Vec<char> = name.chars().collect();
  let mut out = String::with_capacity(name.len() + 4);

  for (i, &c) in chars.iter().enumerate() {
    if c.is_uppercase() {
      let previous_is_lower = i > 0 && chars[i - 1].is_lowercase();
      let next_is_lower = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
      if i > 0 && (previous_is_lower || next_is_lower) {
        out.push('_');
      }
      out.extend(c.to_lowercase());
    } else {
      out.push(c);
    }
  }
  out
}

pub(crate) fn ext_trait_ident(status: &Ident) -> Ident {
  Ident::new(&format!("{status}Ext"), status.span())
}
