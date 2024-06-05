const express = require('express');
const app = express();
const port = 3000;

// Fake data for the activity feed
const activityFeed = [
  {
    id: 1000,
    title: 'New Photo Uploaded',
    body: 'Alice uploaded a new photo to her album.'
  },
  {
    id: 2000,
    title: 'Comment on Post',
    body: "Bob commented on Charlie's post."
  },
  {
    id: 13,
    title: 'Status Update',
    body: 'Charlie updated their status: "Excited about the new project!"'
  }
];

app.get('/feed', (req, res) => {
  res.json(activityFeed);
// Fake data for users
const users = [
  {
    id: 101,
    name: 'Alice Smith'
  },
  {
    id: 102,
    name: 'Bob Johnson'
  },
  {
    id: 103,
    name: 'Charlie Brown'
  }
];

app.get('/search', (req, res) => {
  // Retrieve the query parameter
  const query = req.query.query?.toLowerCase() || '';

  // Filter tasks based on the query
  const filteredTasks = tasks.filter(task =>
    task.description.toLowerCase().includes(query)
  ).sort((a, b) => a.description.localeCompare(b.description));

  // Filter users based on the query
  const filteredUsers = users.filter(user =>
    user.name.toLowerCase().includes(query)
  ).sort((a, b) => a.name.localeCompare(b.name));

  // Return both sets of results
  res.json({ tasks: filteredTasks, users: filteredUsers });
});

app.listen(port, () => {
  console.log(`Server running on port ${port}`);
});
