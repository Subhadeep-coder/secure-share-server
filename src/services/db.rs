use actix_web::Error;
use futures_util::{StreamExt, TryStreamExt};
use mongodb::{
    bson::{doc, oid::ObjectId, Bson, DateTime, Document, Regex},
    results::{InsertOneResult, UpdateResult},
    Client, Collection,
};

use crate::models::{file_model::File, share_link_model::ShareLink, user_model::User};

pub struct Database {
    user: Collection<User>,
    file: Collection<File>,
    share_link: Collection<ShareLink>,
}

impl Database {
    pub async fn init(db_url: String) -> Self {
        let client: Client = Client::with_uri_str(db_url).await.unwrap();
        let db: mongodb::Database = client.database("file");

        let user: Collection<User> = db.collection("user");
        let file: Collection<File> = db.collection("file");
        let share_link: Collection<ShareLink> = db.collection("share_link");

        Database {
            user,
            file,
            share_link,
        }
    }

    pub async fn create_user(
        &self,
        name: String,
        email: String,
        password: String,
    ) -> Result<InsertOneResult, Error> {
        let user = User {
            _id: ObjectId::new(), // Generate a new ObjectId
            username: name,
            email,
            password,
            public_key: "".to_string(),
            created_at: DateTime::now(), // Set current date and time
            updated_at: DateTime::now(), // Set current date and time
        };

        let result: InsertOneResult = self
            .user
            .insert_one(user)
            .await
            .ok()
            .expect("Error creating user");

        Ok(result)
    }

    pub async fn get_user(&self, email: String) -> Result<User, Error> {
        let filter = doc! {"email":email};

        let exists_user: Option<User> = self
            .user
            .find_one(filter)
            .await
            .ok()
            .expect("Error fetching data");

        exists_user.ok_or_else(|| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "User not found",
            ))
        })
    }

    pub async fn update_public_key(
        &self,
        id: Bson,
        public_key: String,
    ) -> Result<UpdateResult, Error> {
        let filter: Document = doc! { "_id": id };
        let update: Document = doc! { "$set": { "public_key": public_key } };

        let update_result: UpdateResult = self
            .user
            .update_one(filter.clone(), update)
            .await
            .ok()
            .expect("Error updating user");

        Ok(update_result)
    }

    pub async fn get_user_by_id(&self, id: Bson) -> Result<User, Error> {
        let filter: Document = doc! { "_id": id };
        // Use await? to handle the Result from find_one
        let fetch_user = self
            .user
            .find_one(filter)
            .await
            .ok()
            .expect("Error while fetching user");

        fetch_user.ok_or_else(|| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "User not found",
            ))
        })
    }

    pub async fn save_file(
        &self,
        file_name: String,
        file_size: i64,
        file_data: Vec<u8>,
        iv: Vec<u8>,
        aes_key: Vec<u8>,
        reciepient_user_id: String,
        user_id: ObjectId,
        password: String,
        expiration_date: DateTime,
    ) -> Result<InsertOneResult, Error> {
        let file = File {
            _id: ObjectId::new(), // Generate a new ObjectId
            user_id,
            file_name,
            file_size,
            encrypted_aes_key: aes_key,
            encrypted_file: file_data,
            iv,
            created_at: DateTime::now(), // Set current date and time
            updated_at: DateTime::now(), // Set current date and time
        };

        let result = self
            .file
            .insert_one(file)
            .await
            .ok()
            .expect("FAILED TO INSERT FILE IN DATABASE");

        // Safely extract the ObjectId from the result.inserted_id
        let file_id = match result.inserted_id {
            Bson::ObjectId(oid) => oid, // Successfully extracted ObjectId
            _ => {
                return Err(actix_web::error::ErrorBadRequest(format!(
                    "Failed to convert bson to objectId"
                )))
            }
        };

        // Safely extract the ObjectId from the reciepient_user_id
        let reciepient_user_id = match ObjectId::parse_str(&reciepient_user_id) {
            Ok(id) => id,
            Err(e) => {
                return Err(actix_web::error::ErrorBadRequest(format!(
                    "Failed to convert to objectid: {}",
                    e.to_string()
                )));
            }
        };

        let share_link = ShareLink {
            _id: ObjectId::new(),
            file_id,
            password,
            reciepents_user_id: reciepient_user_id,
            created_at: DateTime::now(), // Set current date and time
            expires_at: expiration_date,
        };
        let _share_result = self
            .share_link
            .insert_one(share_link)
            .await
            .ok()
            .expect("Failed to save the share document");

        Ok(result)
    }

    pub async fn get_shared(
        &self,
        share_id: ObjectId,
        user_id: ObjectId,
    ) -> Result<ShareLink, Error> {
        let filter: Document = doc! { "reciepents_user_id": user_id,"_id": share_id };

        let result = self
            .share_link
            .find_one(filter)
            .await
            .ok()
            .expect("Couldn't find the shared_file");

        result.ok_or_else(|| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Shared file not found",
            ))
        })
    }

    pub async fn get_file(&self, file_id: Bson) -> Result<File, Error> {
        let filter: Document = doc! { "_id": file_id };
        // Use await? to handle the Result from find_one
        let fetch_file = self
            .file
            .find_one(filter)
            .await
            .ok()
            .expect("Error while fetching user");

        fetch_file.ok_or_else(|| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not found",
            ))
        })
    }

    pub async fn get_sent_files(
        &self,
        user_id: String,
        page: u32,
        limit: usize,
    ) -> Result<Vec<(File, String)>, Error> {
        // Safely extract the ObjectId from the reciepient_user_id
        let user_id = match ObjectId::parse_str(&user_id) {
            Ok(id) => id,
            Err(e) => {
                return Err(actix_web::error::ErrorBadRequest(format!(
                    "Failed to convert to objectid: {}",
                    e.to_string()
                )));
            }
        };

        let filter = doc! {"user_id": user_id};
        let offset = (page - 1) * limit as u32;
        // Execute the query and get the cursor
        let cursor = self
            .file
            .find(filter)
            .skip(offset.into())
            .limit(limit.try_into().unwrap())
            .await
            .ok()
            .expect("Failed to get files");

        // Collect files into a vector
        let mut files: Vec<(File, String)> = Vec::new();
        // Use the StreamExt trait to process the cursor asynchronously
        let mut stream = cursor.into_stream();
        while let Some(result) = stream.next().await {
            match result {
                Ok(file) => {
                    let filter = doc! {"file_id": file._id};

                    match self.share_link.find_one(filter).await {
                        Ok(Some(share_link)) => {
                            files.push((file, share_link._id.to_string()));
                        }
                        Ok(None) => {
                            return Err(actix_web::error::ErrorServiceUnavailable(format!(
                                "Unable to fetch file"
                            )));
                        }
                        Err(e) => {
                            return Err(actix_web::error::ErrorServiceUnavailable(format!(
                                "Unable to fetch file: {}",
                                e
                            )));
                        }
                    }
                } // Push the file if successful
                Err(e) => {
                    // Log the error if necessary
                    actix_web::error::ErrorServiceUnavailable(format!(
                        "Unable to fetch file: {}",
                        e.to_string()
                    ));
                    // Optionally, you could handle the error further here
                }
            }
        }

        Ok(files)
    }

    pub async fn get_recieve_files(
        &self,
        user_id: String,
        page: u32,
        limit: usize,
    ) -> Result<Vec<(File, String)>, Error> {
        // Safely extract the ObjectId from the reciepient_user_id
        let user_id = match ObjectId::parse_str(&user_id) {
            Ok(id) => id,
            Err(e) => {
                return Err(actix_web::error::ErrorBadRequest(format!(
                    "Failed to convert to objectid: {}",
                    e.to_string()
                )));
            }
        };

        let filter = doc! {"reciepents_user_id": user_id};
        let offset = (page - 1) * limit as u32;
        let share_links = self
            .share_link
            .find(filter.clone())
            .skip(offset.into())
            .limit(limit.try_into().unwrap())
            .await
            .ok()
            .expect("Failed to fetch shared links");

        // Collect files into a vector
        let mut files: Vec<(File, String)> = Vec::new();
        // Use the StreamExt trait to process the cursor asynchronously
        let mut stream = share_links.into_stream();
        while let Some(result) = stream.next().await {
            match result {
                Ok(share_link) => {
                    let filter = doc! {"_id": share_link.file_id};
                    // Attempt to find the file and handle errors appropriately
                    match self.file.find_one(filter).await {
                        Ok(Some(file_data)) => {
                            files.push((file_data, share_link._id.to_string()));
                            // Push the file if successful
                        }
                        Ok(None) => {
                            return Err(actix_web::error::ErrorServiceUnavailable(format!(
                                "Unable to fetch file"
                            )));
                            // Optionally handle the case where the file does not exist
                        }
                        Err(e) => {
                            return Err(actix_web::error::ErrorServiceUnavailable(format!(
                                "Unable to fetch file: {}",
                                e
                            )));
                        }
                    }
                } // Push the file if successful
                Err(e) => {
                    // Log the error if necessary
                    actix_web::error::ErrorServiceUnavailable(format!(
                        "Unable to fetch shared_link: {}",
                        e.to_string()
                    ));
                    // Optionally, you could handle the error further here
                }
            }
        }

        Ok(files)
    }

    pub async fn get_recipients_email_by_file_id(&self, file_id: String) -> Result<User, Error> {
        // Safely extract the ObjectId from the file_id
        let file_id = match ObjectId::parse_str(&file_id) {
            Ok(id) => id,
            Err(e) => {
                return Err(actix_web::error::ErrorBadRequest(format!(
                    "Failed to convert to objectid: {}",
                    e.to_string()
                )));
            }
        };

        let filter = doc! {"file_id": file_id};
        let shared_link = match self
            .share_link
            .find_one(filter)
            .await
            .ok()
            .expect("Failed to fetch shared link")
        {
            Some(link) => link,
            None => {
                return Err(actix_web::error::ErrorBadRequest(format!(
                    "Failed to get shared link"
                )));
            }
        };

        let filter = doc! {"_id": shared_link.reciepents_user_id };
        let user = match self
            .user
            .find_one(filter)
            .await
            .ok()
            .expect("Failed to fetch user")
        {
            Some(user) => user,
            None => {
                return Err(actix_web::error::ErrorBadRequest(format!(
                    "Failed to get user"
                )));
            }
        };

        Ok(user)
    }

    pub async fn delete_file_by_share_id(&self, share_id: String) -> Result<bool, Error> {
        // Safely extract the ObjectId from the share_id
        let share_id = match ObjectId::parse_str(&share_id) {
            Ok(id) => id,
            Err(e) => {
                return Err(actix_web::error::ErrorBadRequest(format!(
                    "Failed to convert to objectid: {}",
                    e.to_string()
                )));
            }
        };
        let filter = doc! {"_id": share_id,};
        let delete_share_link = match self
            .share_link
            .find_one_and_delete(filter)
            .await
            .ok()
            .expect("Failed to delete shared link")
        {
            Some(shared_link) => shared_link,
            None => {
                return Err(actix_web::error::ErrorBadRequest(format!(
                    "Failed to delete shared link"
                )));
            }
        };

        let filter = doc! {"_id": delete_share_link.file_id};

        let _deleted_file = match self
            .file
            .find_one_and_delete(filter)
            .await
            .ok()
            .expect("Failed to delete file")
        {
            Some(file) => file,
            None => {
                return Err(actix_web::error::ErrorBadRequest(format!(
                    "Failed to delete file"
                )));
            }
        };
        Ok(true)
    }

    pub async fn get_share_link_doc(&self, share_id: String) -> Result<File, Error> {
        // Safely extract the ObjectId from the share_id
        let share_id = match ObjectId::parse_str(&share_id) {
            Ok(id) => id,
            Err(e) => {
                return Err(actix_web::error::ErrorBadRequest(format!(
                    "Failed to convert to objectid: {}",
                    e.to_string()
                )));
            }
        };
        let filter = doc! {"_id": share_id};
        let share_link = match self
            .share_link
            .find_one(filter)
            .await
            .ok()
            .expect("Failed to fetch shared link")
        {
            Some(shared_link) => shared_link,
            None => {
                return Err(actix_web::error::ErrorBadRequest(format!(
                    "Failed to delete shared link"
                )));
            }
        };

        let filter = doc! {"_id": share_link.file_id};

        let file = match self
            .file
            .find_one(filter)
            .await
            .ok()
            .expect("Failed to delete file")
        {
            Some(file) => file,
            None => {
                return Err(actix_web::error::ErrorBadRequest(format!(
                    "Failed to delete file"
                )));
            }
        };

        Ok(file)
    }

    pub async fn search_user(&self, email_text: String) -> Result<Vec<User>, Error> {
        // Create a regex pattern that matches email addresses containing the substring
        let filter = doc! {
            "email": Regex {
                pattern: email_text,
                options: "i".to_string(), // 'i' for case-insensitive matching
            }
        };

        // Perform the search
        let cursor = match self.user.find(filter).await {
            Ok(cursor) => cursor,
            Err(e) => {
                return Err(actix_web::error::ErrorBadRequest(format!(
                    "Failed to fetch users: {}",
                    e.to_string()
                )));
            }
        };

        // Collect the results into a vector
        let users: Vec<User> = match cursor.try_collect().await {
            Ok(users) => users,
            Err(e) => {
                return Err(actix_web::error::ErrorBadRequest(format!(
                    "Failed to fetch users: {}",
                    e.to_string()
                )));
            }
        };

        Ok(users) // Return the list of users found
    }

    pub async fn delete_expired_files(&self) -> Result<(), Error> {
        // Current time in UTC
        let now: DateTime = DateTime::now();

        let filter = doc! {"expires_at":{"$lt": now}};
        let cursor = self
            .share_link
            .find(filter)
            .await
            .ok()
            .expect("Failed to fetch expired docs");
        let mut file_ids: Vec<ObjectId> = Vec::new();
        let mut share_ids: Vec<ObjectId> = Vec::new();
        let mut stream = cursor.into_stream();
        while let Some(result) = stream.next().await {
            match result {
                Ok(shared_link) => {
                    share_ids.push(shared_link._id);
                    let filter = doc! {"_id": shared_link.file_id};
                    let file = self
                        .file
                        .find_one(filter)
                        .await
                        .ok()
                        .expect("Failed to fetch file")
                        .expect("Unable to fetch");
                    file_ids.push(file._id);
                }
                Err(e) => {
                    actix_web::error::ErrorServiceUnavailable(format!(
                        "Unable to fetch share_link: {}",
                        e.to_string()
                    ));
                }
            }
        }

        let delete_shared_links_result = self
            .share_link
            .delete_many(doc! {"expires_at":{"$lt":now}})
            .await
            .ok()
            .expect("Failed to delete the shared links");

        let delete_files_result = self
            .file
            .delete_many(doc! {"_id":{"$in":file_ids}})
            .await
            .ok()
            .expect("Failed to delete files");

        println!(
            "Successfully deleted {} expired shared links.",
            delete_shared_links_result.deleted_count
        );
        println!(
            "Successfully deleted {} expired files.",
            delete_files_result.deleted_count
        );

        Ok(())
    }
}
