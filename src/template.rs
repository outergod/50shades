// This file is part of 50shades.
//
// Copyright 2019 Communicatio.Systems GmbH
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use failure::Error;
use handlebars::{
    Context, Handlebars, Helper, HelperResult, JsonRender, JsonValue as Json, Output,
    RenderContext, RenderError,
};
use serde::Serialize;

const TEMPLATE_KEY: &str = "50shades";

fn default_helper(
    helper: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let (value, default) = match helper.params().as_slice() {
        [value, default] => (value.value(), default.render()),
        _ => {
            return Err(RenderError::new(
                "`default` helper must be invoked with two parameters, `value` and `default`",
            ))
        }
    };

    match value {
        Json::Null => out.write(&default)?,
        _ => out.write(&value.render())?,
    }

    Ok(())
}

pub fn compile(template: &str) -> Result<Handlebars, Error> {
    let mut handlebars = Handlebars::new();
    handlebars.register_helper("default", Box::new(default_helper));
    handlebars.register_template_string(TEMPLATE_KEY, template)?;
    Ok(handlebars)
}

pub fn render<S: Serialize>(handlebars: &Handlebars, data: &S) -> Result<String, Error> {
    Ok(handlebars.render(TEMPLATE_KEY, data)?)
}

#[cfg(test)]
mod test {
    use super::default_helper;
    use handlebars::Handlebars;
    use std::collections::HashMap;

    #[test]
    fn test_default_helper() {
        let mut r = Handlebars::new();

        r.register_helper("default", Box::new(default_helper));

        assert!(r
            .register_template_string("a", "{{default foo \"baz\"}}")
            .is_ok());

        assert!(r
            .register_template_string("b", "{{default not-found \"baz\"}}")
            .is_ok());

        assert!(r.register_template_string("c", "{{default}}").is_ok());
        assert!(r.register_template_string("d", "{{default foo}}").is_ok());

        let mut context = HashMap::<&str, &str>::new();
        context.insert("foo", "bar");

        assert_eq!(r.render("a", &context).unwrap(), "bar");
        assert_eq!(r.render("b", &context).unwrap(), "baz");
        assert!(r.render("c", &context).is_err());
        assert!(r.render("d", &context).is_err());
    }
}
