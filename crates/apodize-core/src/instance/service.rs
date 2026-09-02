use crate::InstanceError;

use super::{Instance, InstanceId, InstanceRepository};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateInstanceRequest {
    pub name: String,
    pub minecraft_version: String,
}

impl CreateInstanceRequest {
    #[must_use]
    pub fn new(name: impl Into<String>, minecraft_version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            minecraft_version: minecraft_version.into(),
        }
    }
}

#[derive(Debug)]
pub struct InstanceService<R> {
    repository: R,
}

impl<R> InstanceService<R>
where
    R: InstanceRepository,
{
    #[must_use]
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn create(&self, request: CreateInstanceRequest) -> Result<Instance, InstanceError> {
        let instance = Instance::new(request.name, request.minecraft_version)?;
        self.repository.create(&instance)?;
        Ok(instance)
    }

    pub fn get(&self, id: &InstanceId) -> Result<Instance, InstanceError> {
        self.repository
            .get(id)?
            .ok_or_else(|| InstanceError::NotFound(id.to_string()))
    }

    pub fn list(&self) -> Result<Vec<Instance>, InstanceError> {
        self.repository.list()
    }

    pub fn remove(&self, id: &InstanceId) -> Result<(), InstanceError> {
        self.repository.delete(id)
    }

    #[must_use]
    pub fn repository(&self) -> &R {
        &self.repository
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::BTreeMap};

    use super::*;

    #[derive(Debug, Default)]
    struct MemoryRepository {
        instances: RefCell<BTreeMap<InstanceId, Instance>>,
    }

    impl InstanceRepository for MemoryRepository {
        fn create(&self, instance: &Instance) -> Result<(), InstanceError> {
            let mut instances = self.instances.borrow_mut();
            if instances.contains_key(instance.id()) {
                return Err(InstanceError::AlreadyExists(instance.id().to_string()));
            }
            instances.insert(instance.id().clone(), instance.clone());
            Ok(())
        }

        fn get(&self, id: &InstanceId) -> Result<Option<Instance>, InstanceError> {
            Ok(self.instances.borrow().get(id).cloned())
        }

        fn list(&self) -> Result<Vec<Instance>, InstanceError> {
            Ok(self.instances.borrow().values().cloned().collect())
        }

        fn delete(&self, id: &InstanceId) -> Result<(), InstanceError> {
            if self.instances.borrow_mut().remove(id).is_none() {
                return Err(InstanceError::NotFound(id.to_string()));
            }
            Ok(())
        }
    }

    #[test]
    fn service_creates_and_loads_instance() {
        let service = InstanceService::new(MemoryRepository::default());
        let created = service
            .create(CreateInstanceRequest::new("Survival", "1.21.8"))
            .expect("create instance");

        let loaded = service.get(created.id()).expect("load instance");
        assert_eq!(loaded, created);
    }

    #[test]
    fn service_rejects_empty_instance_name() {
        let service = InstanceService::new(MemoryRepository::default());
        let error = service
            .create(CreateInstanceRequest::new("   ", "1.21.8"))
            .expect_err("empty name must fail");

        assert!(matches!(error, InstanceError::EmptyName));
    }

    #[test]
    fn service_rejects_empty_minecraft_version() {
        let service = InstanceService::new(MemoryRepository::default());
        let error = service
            .create(CreateInstanceRequest::new("Survival", "  "))
            .expect_err("empty version must fail");

        assert!(matches!(error, InstanceError::EmptyMinecraftVersion));
    }
}
