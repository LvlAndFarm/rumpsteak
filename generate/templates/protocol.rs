use ::futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor, try_join
};
#[allow(unused_imports)]
use ::rumpsteak::{
    channel::Bidirectional, session, Branch, End, Message, Receive, Role, Roles, Select, Send, try_session
};

use std::error::Error;

pub type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

#[derive(Roles)]
#[allow(dead_code)]
pub struct Roles {
{%- for role in roles %}
    pub {{ role.snake }}: {{ role.camel }},
{%- endfor %}
}
{% for role in roles %}
#[derive(Role)]
#[message(Label)]
pub struct {{ role.camel }} {
{%- for index in role.routes.iter() %}
    {%- let route = roles[index.0] %}
    #[route({{ route.camel }})]
    {{ route.snake }}: Channel,
{%- endfor %}
}
{% endfor %}
#[derive(Message)]
pub enum Label {
{%- for label in labels %}
    {{ label.camel }}({{ label.camel }}),
{%- endfor %}
}
{% for label in labels %}
pub struct {{ label.camel }}{% if !label.parameters.is_empty() -%}
    (pub {{ label.parameters|join(", pub") }})
{%- endif %};
{% endfor %}
{%- for role in roles %}
{%- for (i, definition) in role.definitions.iter().rev().enumerate() %}
{%- let node = role.nodes[definition.node] %}
#[session]
{%- match definition.body %}
{%- when DefinitionBody::Type with { safe, ty } %}
{%- if safe|copy_bool %}
pub type {{ camel }}{{ role.camel }}{% if i > 0 -%}{{ node }}{%- endif %} = {{ ty|ty(camel, role, roles, labels) }};
{%- else %}
pub struct {{ camel }}{{ role.camel }}{% if i > 0 -%}{{ node }}{%- endif %}({{ ty|ty(camel, role, roles, labels) }});
{%- endif %}
{%- when DefinitionBody::Choice with (choices) %}
pub enum {{ camel }}{{ role.camel }}{{ node }} {
{%- for choice in choices %}
    {%- let label = labels[choice.label] %}
    {{ label.camel }}({{ label.camel }}, {{ choice.ty|ty(camel, role, roles, labels) }}),
{%- endfor %}
}
{%- endmatch %}
{% endfor %}
{%- endfor %}
