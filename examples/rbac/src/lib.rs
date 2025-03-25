mod user;

use serde::{Deserialize, Serialize};

#[test]
fn test() {
    let role: crate::Role = serde_json::from_str(
        r#"
			{
				"rules": [
					{
						"User": {
							"scopes": [ {"Group": 1} ],
							"permissions": [ "Create", "Delete" ]
						}
					},
					{
						"User": {
							"scopes": "*",
							"permissions": [ "View" ]
						}
					}
				]
			}
		"#,
    )
    .unwrap();

    let user_a = user::User {
        id: 1,
        name: "John".into(),
        group_ids: vec![1, 2],
    };

    let user_b = user::User {
        id: 2,
        name: "Daniel".into(),
        group_ids: vec![2],
    };

    use crate::RoleExt as _;
    use user::UserPermission;

    assert_eq!(true, role.allows(&user_a, UserPermission::Create));
    assert_eq!(true, role.allows(&user_a, UserPermission::Delete));
    assert_eq!(true, role.allows(&user_a, UserPermission::View));

    assert_eq!(false, role.allows(&user_b, UserPermission::Create));
    assert_eq!(false, role.allows(&user_b, UserPermission::Delete));
    assert_eq!(true, role.allows(&user_b, UserPermission::View));
}

pub trait AccessControl {
    type Scope;
    type Permission: PartialEq;

    fn within_scope(&self, scope: &Self::Scope) -> bool;
}

pub trait RoleExt<T: AccessControl> {
    fn allows(&self, subject: &T, permission: T::Permission) -> bool;
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Scopes<T: AccessControl>
where
    for<'a> T::Scope: Serialize + Deserialize<'a>,
{
    #[serde(rename = "*")]
    Any,

    #[serde(untagged)]
    Constrained(Vec<T::Scope>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuleInner<T: AccessControl>
where
    for<'a> T::Scope: std::fmt::Debug + Clone + Serialize + Deserialize<'a>,
    for<'a> T::Permission: Serialize + Deserialize<'a>,
{
    scopes: Scopes<T>,
    permissions: Vec<T::Permission>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Role {
    rules: Vec<Rule>,
}

traitable::generate!(
    (AccessControl) => {
        #[derive(Debug, Clone, Deserialize, Serialize)]
        pub enum Rule {
            $($ty (RuleInner<$ty_full>) )*
        }

        $(
            impl RoleExt<$ty_full> for Role {
                fn allows(&self, subject: &$ty_full, permission: <$ty_full as AccessControl>::Permission) -> bool {
                    for rule in &self.rules {
                        #[allow(irrefutable_let_patterns)]
                        let Rule::$ty(rule) = rule else {
                            continue;
                        };

                        if !rule.permissions.contains(&permission) {
                            continue;
                        }

                        match &rule.scopes {
                            Scopes::Any => return true,
                            Scopes::Constrained(scopes) => if scopes.iter().all(|s| subject.within_scope(s)) {
                                return true
                            }
                        }
                    }

                    false
                }
            }
        )*
    }
);
