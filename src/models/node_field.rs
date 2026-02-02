use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NodeField {
    pub field_name: String,
    pub field_type: String,
    pub cardinality: i32,
    pub settings: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NodeFieldInstance {
    pub id: u32,
    pub field_name: String,
    pub node_type: String,
    pub label: String,
    pub description: Option<String>,
    pub required: i8,
    pub weight: i32,
    pub widget_type: Option<String>,
    pub widget_settings: Option<String>,
    pub display_settings: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NodeFieldData {
    pub id: u32,
    pub nid: u32,
    pub vid: u32,
    pub field_name: String,
    pub delta: u32,
    pub value_text: Option<String>,
    pub value_int: Option<i64>,
    pub value_float: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FieldInstanceJoined {
    pub id: u32,
    pub field_name: String,
    pub node_type: String,
    pub label: String,
    pub description: Option<String>,
    pub required: i8,
    pub weight: i32,
    pub widget_type: Option<String>,
    pub field_type: String,
    pub cardinality: i32,
    pub settings: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInstanceWithValue {
    pub field_name: String,
    pub field_type: String,
    pub label: String,
    pub description: Option<String>,
    pub required: i8,
    pub weight: i32,
    pub widget_type: Option<String>,
    pub cardinality: i32,
    pub settings: Option<String>,
    pub values: Vec<FieldValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldValue {
    pub delta: u32,
    pub value_text: Option<String>,
    pub value_int: Option<i64>,
    pub value_float: Option<f64>,
}

impl NodeField {
    pub async fn find_by_name(
        pool: &MySqlPool,
        field_name: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, NodeField>("SELECT * FROM node_field WHERE field_name = ?")
            .bind(field_name)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(
        pool: &MySqlPool,
        field_name: &str,
        field_type: &str,
        cardinality: i32,
        settings: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO node_field (field_name, field_type, cardinality, settings) VALUES (?, ?, ?, ?)",
        )
        .bind(field_name)
        .bind(field_type)
        .bind(cardinality)
        .bind(settings)
        .execute(pool)
        .await?;

        Ok(())
    }
}

impl NodeFieldInstance {
    pub async fn for_node_type(
        pool: &MySqlPool,
        node_type: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, NodeFieldInstance>(
            "SELECT * FROM node_field_instance WHERE node_type = ? ORDER BY weight, label",
        )
        .bind(node_type)
        .fetch_all(pool)
        .await
    }

    pub async fn create(
        pool: &MySqlPool,
        field_name: &str,
        node_type: &str,
        label: &str,
        description: Option<&str>,
        required: bool,
        weight: i32,
        widget_type: &str,
    ) -> Result<u32, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO node_field_instance (field_name, node_type, label, description, required, weight, widget_type)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(field_name)
        .bind(node_type)
        .bind(label)
        .bind(description)
        .bind(if required { 1i8 } else { 0i8 })
        .bind(weight)
        .bind(widget_type)
        .execute(pool)
        .await?;

        Ok(result.last_insert_id() as u32)
    }

    pub async fn with_field_info(
        pool: &MySqlPool,
        node_type: &str,
    ) -> Result<Vec<FieldInstanceWithValue>, sqlx::Error> {
        let rows = sqlx::query_as::<_, FieldInstanceJoined>(
            "SELECT nfi.id, nfi.field_name, nfi.node_type, nfi.label, nfi.description,
                    nfi.required, nfi.weight, nfi.widget_type,
                    nf.field_type, nf.cardinality, nf.settings
             FROM node_field_instance nfi
             INNER JOIN node_field nf ON nfi.field_name = nf.field_name
             WHERE nfi.node_type = ?
             ORDER BY nfi.weight, nfi.label",
        )
        .bind(node_type)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| FieldInstanceWithValue {
                field_name: row.field_name,
                field_type: row.field_type,
                label: row.label,
                description: row.description,
                required: row.required,
                weight: row.weight,
                widget_type: row.widget_type,
                cardinality: row.cardinality,
                settings: row.settings,
                values: vec![],
            })
            .collect())
    }
}

impl NodeFieldData {
    pub async fn for_revision(
        pool: &MySqlPool,
        vid: u32,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, NodeFieldData>(
            "SELECT * FROM node_field_data WHERE vid = ? ORDER BY field_name, delta",
        )
        .bind(vid)
        .fetch_all(pool)
        .await
    }

    pub async fn save(
        pool: &MySqlPool,
        nid: u32,
        vid: u32,
        field_name: &str,
        delta: u32,
        value_text: Option<String>,
        value_int: Option<i64>,
        value_float: Option<f64>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO node_field_data (nid, vid, field_name, delta, value_text, value_int, value_float)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON DUPLICATE KEY UPDATE value_text = VALUES(value_text), value_int = VALUES(value_int), value_float = VALUES(value_float)",
        )
        .bind(nid)
        .bind(vid)
        .bind(field_name)
        .bind(delta)
        .bind(value_text)
        .bind(value_int)
        .bind(value_float)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete_for_revision(
        pool: &MySqlPool,
        vid: u32,
        field_name: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM node_field_data WHERE vid = ? AND field_name = ?")
            .bind(vid)
            .bind(field_name)
            .execute(pool)
            .await?;

        Ok(())
    }
}

pub async fn get_fields_with_values(
    pool: &MySqlPool,
    node_type: &str,
    vid: u32,
) -> Result<Vec<FieldInstanceWithValue>, sqlx::Error> {
    let mut fields = NodeFieldInstance::with_field_info(pool, node_type).await?;
    let data = NodeFieldData::for_revision(pool, vid).await?;

    let mut data_map: HashMap<String, Vec<FieldValue>> = HashMap::new();
    for d in data {
        data_map
            .entry(d.field_name.clone())
            .or_default()
            .push(FieldValue {
                delta: d.delta,
                value_text: d.value_text,
                value_int: d.value_int,
                value_float: d.value_float,
            });
    }

    for field in &mut fields {
        if let Some(values) = data_map.remove(&field.field_name) {
            field.values = values;
        }
    }

    Ok(fields)
}

pub async fn save_field_values(
    pool: &MySqlPool,
    nid: u32,
    vid: u32,
    node_type: &str,
    form_data: &HashMap<String, String>,
) -> Result<(), sqlx::Error> {
    let fields = NodeFieldInstance::with_field_info(pool, node_type).await?;

    for field in fields {
        NodeFieldData::delete_for_revision(pool, vid, &field.field_name).await?;

        if field.cardinality == 1 {
            let key = format!("field_{}", field.field_name);
            if let Some(value) = form_data.get(&key) {
                if !value.is_empty() {
                    let (text, int_val, float_val) = parse_field_value(&field.field_type, value);
                    NodeFieldData::save(pool, nid, vid, &field.field_name, 0, text, int_val, float_val).await?;
                }
            }
        } else {
            for delta in 0..10u32 {
                let key = format!("field_{}_{}", field.field_name, delta);
                if let Some(value) = form_data.get(&key) {
                    if !value.is_empty() {
                        let (text, int_val, float_val) = parse_field_value(&field.field_type, value);
                        NodeFieldData::save(pool, nid, vid, &field.field_name, delta, text, int_val, float_val).await?;
                    }
                }
            }
        }
    }

    Ok(())
}

fn parse_field_value(field_type: &str, value: &str) -> (Option<String>, Option<i64>, Option<f64>) {
    match field_type {
        "integer" | "number_integer" => {
            let int_val = value.parse::<i64>().ok();
            (None, int_val, None)
        }
        "decimal" | "float" | "number_decimal" => {
            let float_val = value.parse::<f64>().ok();
            (None, None, float_val)
        }
        "boolean" | "checkbox" => {
            let int_val = if value == "1" || value.to_lowercase() == "true" {
                Some(1i64)
            } else {
                Some(0i64)
            };
            (None, int_val, None)
        }
        _ => (Some(value.to_string()), None, None),
    }
}
