use crate::world::entity::EntityType;
use crate::world::instance::Instance;
use std::borrow::Cow;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// Summary returned after exporting a `.loci` replay to CSV.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvExportReport {
    pub total_ticks: u64,
    pub rows_written: usize,
    pub output_path: String,
}

impl std::fmt::Display for CsvExportReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Exported {} ticks ({} entity rows) to '{}'",
            self.total_ticks, self.rows_written, self.output_path
        )
    }
}

fn escape_csv_field(field: &str) -> Cow<'_, str> {
    if field.contains([',', '"', '\n', '\r']) {
        let mut out = String::with_capacity(field.len() + 2);
        out.push('"');
        for ch in field.chars() {
            if ch == '"' {
                out.push('"');
            }
            out.push(ch);
        }
        out.push('"');
        Cow::Owned(out)
    } else {
        Cow::Borrowed(field)
    }
}

fn entity_type_label(t: &EntityType) -> &'static str {
    match t {
        EntityType::Player => "Player",
        EntityType::NPC => "NPC",
        EntityType::Prop => "Prop",
        EntityType::Projectile => "Projectile",
    }
}

/// Shared buffered CSV writer for per-tick, per-entity simulation telemetry.
/// Used by both offline `--replay --export-csv` and live `--record-csv`.
pub struct CsvTelemetryWriter {
    writer: BufWriter<File>,
    rows_written: usize,
}

impl CsvTelemetryWriter {
    pub fn create<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        if let Some(parent) = path.as_ref().parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        writeln!(
            writer,
            "tick,entity_id,name,entity_type,pos_x,pos_y,vel_x,vel_y"
        )?;
        Ok(Self {
            writer,
            rows_written: 0,
        })
    }

    pub fn write_instance_tick(&mut self, tick: u64, instance: &Instance) -> std::io::Result<()> {
        for entity in instance.entities.values() {
            writeln!(
                self.writer,
                "{},{},{},{},{:.4},{:.4},{:.4},{:.4}",
                tick,
                entity.id,
                escape_csv_field(&entity.name),
                entity_type_label(&entity.entity_type),
                entity.position.x.to_num::<f64>(),
                entity.position.y.to_num::<f64>(),
                entity.velocity.x.to_num::<f64>(),
                entity.velocity.y.to_num::<f64>(),
            )?;
            self.rows_written += 1;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }

    pub fn rows_written(&self) -> usize {
        self.rows_written
    }
}
