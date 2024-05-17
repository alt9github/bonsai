#![allow(dead_code, unused_imports, unused_variables)]
use std::fmt::Display;
use std::fmt::Write;
use std::fmt::{self};

use petgraph::visit::EdgeRef;
use petgraph::visit::GraphProp;
use petgraph::visit::IntoEdgeReferences;
use petgraph::visit::IntoNodeReferences;
use petgraph::visit::NodeIndexable;
use petgraph::visit::NodeRef;

pub struct Mermaid<'a, G>
    where
        G: IntoEdgeReferences + IntoNodeReferences,
{
    graph: G,
    get_edge_attributes: &'a dyn Fn(G, G::EdgeRef) -> String,
    get_node_attributes: &'a dyn Fn(G, G::NodeRef) -> String,
    config: Configs,
}

static EDGE: [&str; 2] = ["---", "-->"];
static INDENT: &str = "    ";

impl<'a, G> Mermaid<'a, G>
    where
        G: IntoNodeReferences + IntoEdgeReferences,
{
    /// Create a `Mermaid` formatting wrapper with default configuration.
    #[inline]
    pub fn new(graph: G) -> Self {
        Self::with_config(graph, &[])
    }

    /// Create a `Mermaid` formatting wrapper with custom configuration.
    #[inline]
    pub fn with_config(graph: G, config: &'a [Config]) -> Self {
        Self::with_attr_getters(graph, config, &|_, _| String::new(), &|_, _| String::new())
    }

    #[inline]
    pub fn with_attr_getters(
        graph: G,
        config: &'a [Config],
        get_edge_attributes: &'a dyn Fn(G, G::EdgeRef) -> String,
        get_node_attributes: &'a dyn Fn(G, G::NodeRef) -> String,
    ) -> Self {
        let config = Configs::extract(config);
        Mermaid {
            graph,
            get_edge_attributes,
            get_node_attributes,
            config,
        }
    }
}

/// `Mermaid` configuration.
///
/// This enum does not have an exhaustive definition (will be expanded)
// TODO: #[non_exhaustive] once MSRV >= 1.40,
// and/or for a breaking change make this something like an EnumSet: https://docs.rs/enumset
#[derive(Debug, PartialEq, Eq)]
pub enum Config {
    /// Use indices for node labels.
    NodeIndexLabel,
    /// Use indices for edge labels.
    EdgeIndexLabel,
    /// Use no edge labels.
    EdgeNoLabel,
    /// Use no node labels.
    NodeNoLabel,
    #[doc(hidden)]
    _Incomplete(()),
}
macro_rules! make_config_struct {
    ($($variant:ident,)*) => {
        #[allow(non_snake_case)]
        #[derive(Default)]
        struct Configs {
            $($variant: bool,)*
        }
        impl Configs {
            #[inline]
            fn extract(configs: &[Config]) -> Self {
                let mut conf = Self::default();
                for c in configs {
                    match *c {
                        $(Config::$variant => conf.$variant = true,)*
                        Config::_Incomplete(()) => {}
                    }
                }
                conf
            }
        }
    }
}
make_config_struct!(NodeIndexLabel, EdgeIndexLabel, EdgeNoLabel, NodeNoLabel,);

impl<'a, G> Mermaid<'a, G>
    where
        G: IntoNodeReferences + IntoEdgeReferences + NodeIndexable + GraphProp,
{
    fn graph_fmt<NF, EF>(&self, f: &mut fmt::Formatter, node_fmt: NF, _edge_fmt: EF) -> fmt::Result
        where
            NF: Fn(&G::NodeWeight, &mut fmt::Formatter) -> fmt::Result,
            EF: Fn(&G::EdgeWeight, &mut fmt::Formatter) -> fmt::Result,
    {
        let g = self.graph;
        writeln!(f, "flowchart TD")?;

        // output all labels
        for node in g.node_references() {
            write!(f, "{}{}[", INDENT, g.to_index(node.id()),)?;
            if !self.config.NodeNoLabel {
                write!(f, "\"")?;
                if self.config.NodeIndexLabel {
                    write!(f, "{}", g.to_index(node.id()))?;
                } else {
                    Escaped(FnFmt(node.weight(), &node_fmt)).fmt(f)?;
                }
                write!(f, "\"")?;
            }
            writeln!(f, "{}]", (self.get_node_attributes)(g, node))?;
        }
        // output all edges
        for edge in g.edge_references() {
            write!(
                f,
                "{}{} {} {}",
                INDENT,
                g.to_index(edge.source()),
                EDGE[g.is_directed() as usize],
                g.to_index(edge.target()),
            )?;
            writeln!(f, "{}", (self.get_edge_attributes)(g, edge))?;
        }

        Ok(())
    }
}

impl<'a, G> fmt::Display for Mermaid<'a, G>
    where
        G: IntoEdgeReferences + IntoNodeReferences + NodeIndexable + GraphProp,
        G::EdgeWeight: fmt::Display,
        G::NodeWeight: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.graph_fmt(f, fmt::Display::fmt, fmt::Display::fmt)
    }
}

impl<'a, G> fmt::Debug for Mermaid<'a, G>
    where
        G: IntoEdgeReferences + IntoNodeReferences + NodeIndexable + GraphProp,
        G::EdgeWeight: fmt::Debug,
        G::NodeWeight: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.graph_fmt(f, fmt::Debug::fmt, fmt::Debug::fmt)
    }
}

/// Escape for Graphviz
struct Escaper<W>(W);

impl<W> fmt::Write for Escaper<W>
    where
        W: fmt::Write,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c)?;
        }
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        match c {
            '"' | '\\' => self.0.write_char('\\')?,
            // \l is for left justified linebreak
            '\n' => return self.0.write_str("\\l"),
            _ => {}
        }
        self.0.write_char(c)
    }
}

/// Pass Display formatting through a simple escaping filter
struct Escaped<T>(T);

impl<T> fmt::Display for Escaped<T>
    where
        T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            writeln!(&mut Escaper(f), "{:#}", &self.0)
        } else {
            write!(&mut Escaper(f), "{}", &self.0)
        }
    }
}

/// Format data using a specific format function
struct FnFmt<'a, T, F>(&'a T, F);

impl<'a, T, F> fmt::Display for FnFmt<'a, T, F>
    where
        F: Fn(&'a T, &mut fmt::Formatter<'_>) -> fmt::Result,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.1(self.0, f)
    }
}

#[cfg(test)]
#[allow(dead_code, unused)]
mod test {
    use std::fmt::Write;

    use petgraph::prelude::Graph;
    use petgraph::visit::NodeRef;

    use super::Config;
    use super::Escaper;
    use super::Mermaid;

    #[test]
    fn test_escape() {
        let mut buff = String::new();
        {
            let mut e = Escaper(&mut buff);
            let _ = e.write_str("\" \\ \n");
        }
        assert_eq!(buff, "\\\" \\\\ \\l");
    }

    fn simple_graph() -> Graph<&'static str, &'static str> {
        let mut graph = Graph::<&str, &str>::new();
        let a = graph.add_node("A");
        let b = graph.add_node("B");
        graph.add_edge(a, b, "edge_label");
        graph
    }
}
